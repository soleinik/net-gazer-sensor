use structopt::StructOpt;
use pnet::datalink;
use std::path::Path;


const KEY_IFACE:&str = "network.iface";
const KEY_REPORTING_URL:&str = "reporting.url";

#[derive(StructOpt, Debug, Clone)]
#[structopt(
    name = "net-gazer",
    about = "network connection capture and analysis daemon"
)]
pub struct OptConf {
    // The number of occurrences of the `v/verbose` flag
    /// Verbose mode (-v(info), -vv(debug), -vvv(trace), etc. default: warn)
    #[structopt(short, long, parse(from_occurrences))]
    pub verbosity: u8,

    // the long option will be translated by default to kebab case,
    // i.e. `--nb-cars`.
    /// target network interface. If not provided - first active interface with ipv4 will be used
    #[structopt(short = "i", long = "iface", env = "NG_IFACE")]
    pub iface: Option<String>,

    /// configuration file
    #[structopt(short = "c", long = "config", env = "NG_CONFIG")]
    pub config_path: Option<String>,

    /// reporting url
    #[structopt(short = "r", long = "reporting", env = "NG_REPORTING")]
    pub reporting_url: Option<String>,

}

impl Default for OptConf{
    fn default() -> Self { OptConf::from_args() }   
}


impl OptConf{

    pub fn load(&mut self, app_name: &str){

        let current_dir = std::env::current_dir().unwrap();
        let current_dir = current_dir.to_str().unwrap();

        //try to load default config
        if self.config_path.is_none(){
            let cfg_file_name = format!("{}.toml", app_name);

            let paths = vec![
                format!("{}/etc/{}/{}",current_dir, app_name, cfg_file_name), 
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
            let settings = config::Config::builder().add_source(config::File::with_name(cfg_file)).build().expect("Failed to load config");

            if self.iface.is_none(){
                self.iface = settings.get_string(KEY_IFACE).ok();
            }    

            if self.reporting_url.is_none(){
                self.reporting_url = settings.get_string(KEY_REPORTING_URL).ok();
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

    pub fn validate(&self) -> crate::AppResult<()>{
        //FIXME - collect all errors
        info!("Validating configuration...");
        if self.iface.is_none(){
            error!("Network interface is not specified!");
            std::process::exit(-1);
        }
        if self.reporting_url.is_none(){
            error!("Reporting url is not specified!");
            std::process::exit(-1);
        }
        Ok(())
    }

}
