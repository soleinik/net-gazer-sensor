use std::sync::mpsc;

use pnet::datalink::{ self, Channel, Config, channel};

use pnet::packet::ethernet::EthernetPacket;

#[macro_use] extern crate log;

extern crate lib_comm;
extern crate lib_plugins;


#[async_std::main]
async fn main() -> std::io::Result<()> {

    std::env::set_var("RUST_BACKTRACE", "1");

    //read command line...
    let mut opt = lib_data::OptConf::default();

    //setup logger...
    match opt.verbosity{
        0 => std::env::set_var("RUST_LOG", "warn"),
        1 => std::env::set_var("RUST_LOG", "info"),
        2 => std::env::set_var("RUST_LOG", "debug"),
        _ => std::env::set_var("RUST_LOG", "trace"),

    }
    env_logger::init();

    //load from file...
    opt.load(env!("CARGO_PKG_NAME"));
    opt.validate().unwrap();

    let iface_name = opt.iface.clone().unwrap();

    let net_iface = 
        datalink::interfaces().into_iter()
            .filter(|iface| iface.is_up())
            .filter(|iface| !iface.ips.is_empty())
            .find(| iface | iface.name == iface_name)
            .unwrap_or_else(|| {
                error!("Invalid Network Interface. No active device '{}'",iface_name);
                std::process::exit(-1);
            });

    let mac = net_iface.mac_address();

    //need network
    let net = net_iface.ips.iter()
        .map(|net| {
            match net{
                ipnetwork::IpNetwork::V4(net)=> Some(net),
                _ => None
            }
        })
        .find(|net| net.is_some()).flatten().unwrap();

    info!("Setting up interceptor on {} [{}]", net_iface.name, mac);
    info!("Detected networks:");
    net_iface.ips.iter()
        .for_each(|net| println!("\tnet:{}", net));

    info!("net:{}", net);
        
    let cfg = Config::default();
    // cfg.fanout = Some(
    //     FanoutOption {
    //     group_id: 123,
    //     fanout_type: FanoutType::CPU,
    //     defrag: true,
    //     rollover: false,
    // });

    //reporting...
    let (comm_sender, comm_receiver): (lib_comm::CommTxChannel,lib_comm::CommRxChannel) = mpsc::channel();
    lib_comm::start(comm_receiver, &opt);


    let plugins = lib_plugins::PluginManager::new(&net_iface, comm_sender);
    if plugins.is_empty(){
        error!("No plugins found! System is not operational - aborting...");
        std::process::exit(-4);
    }


    info!("About to create ethernet link channel...");
    let (_, mut rx) = match channel(&net_iface, cfg) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            error!("Unable to open data link channel! Unexpected data link channel type");
            std::process::exit(-2);
        }
        Err(e) => {
            error!("Unable to open data link channel! Error:{}", e);
            std::process::exit(-3);
        }
    };


    info!("Starting listener loop...");
    loop{
        if let Ok(data) = rx.next(){ //this will timeout, as configured
            match EthernetPacket::new(data){
                Some(ethernet_packet) => {
                    plugins.process(&ethernet_packet);
                }
                None => continue
            }
        }
    }
}
