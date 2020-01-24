use structopt::StructOpt;
use pnet::datalink;
use std::path::Path;


const KEY_IFACE:&str = "network.iface";


#[derive(StructOpt, Debug)]
#[structopt(
    name = "net-gazer",
    about = "network connection capture and analysis daemon"
)]
pub struct OptConf {
    // The number of occurrences of the `v/verbose` flag
    /// Verbose mode (-v(info), -vv(debug), -vvv(trace), etc.)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbosity: u8,

    // the long option will be translated by default to kebab case,
    // i.e. `--nb-cars`.
    /// target network interface. If not provided - first active interface with ipv4 will be used
    #[structopt(short = "i", long = "iface", env = "NG_IFACE")]
    pub iface: Option<String>,

    /// configuration file
    #[structopt(short = "c", long = "config", env = "NG_CONFIG")]
    pub config_path: Option<String>
}




impl OptConf{
    pub fn new() -> Self{
        //load command line
        OptConf::from_args()
    }

    pub fn load(&mut self){

        //try to load default config
        if self.config_path.is_none(){
            let app_name = env!("CARGO_PKG_NAME");
            let cfg_file_name = format!("{}.toml", app_name);

            let paths = vec![
                format!("./etc/{}/{}",app_name, cfg_file_name), 
                //user home?
                format!("/usr/local/etc/{}/{}", app_name, cfg_file_name), 
                format!("/etc/{}/{}", app_name, cfg_file_name)
            ];

            self.config_path = paths.iter()
                .map(|n| Path::new(n))
                .filter(|p| p.exists())
                .find(|p|p.is_file())
                .map( |p|p.to_str().unwrap().to_owned());
        }

        if let Some(cfg_file) = self.config_path.clone(){

            let cfg_file = Path::new(&cfg_file).canonicalize().unwrap();
            let cfg_file = cfg_file.to_str().unwrap();
            info!("Loading configuration from {}...", cfg_file);
            let mut settings = config::Config::default();
            settings.merge(config::File::with_name(cfg_file)).unwrap();

            if self.iface.is_none(){
                self.iface = settings.get_str(KEY_IFACE).ok();
            }    
        }

        if self.iface.is_none() {
            let interfaces = datalink::interfaces();
            if let Some(nface) = interfaces.into_iter()
                .filter(|iface| iface.is_up() && !iface.is_loopback() && !iface.is_point_to_point())
                .find(|iface| !iface.ips.is_empty()){
                self.iface = Some(nface.name);
            }
        }
    }
}
