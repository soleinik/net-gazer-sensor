extern crate libloading;
#[macro_use] extern crate log;
use libloading::{Library, Symbol};
use net_gazer_core::{Plugin, CoreSender};
use pnet::packet::ethernet::EthernetPacket;


#[derive(Default)]
pub struct PluginManager<'p,'d>{
    pub plugins: Vec<Box<dyn Plugin<'p, EthernetPacket<'d>>>>,
    pub libraries: Vec<Library>

}

const LIB_LOCATION: &str = "/usr/local/lib/net-gazer";

type PluginCreate<'p,'d> = unsafe fn() -> *mut dyn Plugin<'p, EthernetPacket<'d>>;

impl <'p,'d> PluginManager<'p, 'd>{

    pub fn new() -> Self{

        let mut p_manager = PluginManager::<'p,'d>::default();

        if let Some(list) = discover(){
            for lib_name in list{
                info!("about to load library \"{}\"...", lib_name);
                match Library::new(lib_name.clone()){
                    Ok(lib) => {
                        info!("library \"{}\" loaded! Looking for plugin entry point...", lib_name);
                        unsafe {
                            match lib.get::<Symbol<PluginCreate>>(b"net_gazer_plugin_new"){
                                Ok(fn_creator) => {
                                    let boxed_raw = fn_creator();
                                    let plugin = Box::from_raw(boxed_raw);
                                    info!("plugin [{}] \"{}\" is found! Initializing...", plugin.get_id(), plugin.get_name());
                                    plugin.on_load();
                                    info!("plugin [{}] \"{}\" is fully operational!", plugin.get_id(), plugin.get_name());
                                    p_manager.libraries.push(lib);
                                    p_manager.plugins.push(plugin);
                                }
                                Err(e) => error!("No plugin entry point found in library[{}]. Error:{}", lib_name, e)
                            }
                        }
                    }
                    Err(e) => error!("Failed to load library[{}]. Error:{}",lib_name, e)
                }
            }
        
        }
        p_manager
    }
    
    pub fn process(&self, tx:&CoreSender, pkt:EthernetPacket<'d>){
        self.plugins.iter().for_each(|p| p.process(tx, &pkt));
    }

}


use std::fs;

fn discover() -> Option<Vec<String>>{
    if let Ok(paths) = fs::read_dir(LIB_LOCATION){
        Some(
            paths
            .filter(|e|e.is_ok())
            .map(|e|e.unwrap())
            .map(|e| e.file_name())
            .map(|e| e.to_str().unwrap().to_owned())
            .filter(|e|e.ends_with(".so"))
            .filter(|e|e.starts_with("libnet_gazer"))
            //.map(|e| format!("{}/{}",LIB_LOCATION, e))
            .collect::<Vec<String>>()
        )
    }else{
        None
    }
}