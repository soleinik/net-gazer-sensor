extern crate libloading;
#[macro_use] extern crate log;
use libloading::{Library, Symbol};
use net_gazer_core::{Plugin, CoreSender};
use pnet::packet::ethernet::EthernetPacket;
use pnet::datalink::NetworkInterface;
use std::sync::Arc;

#[derive(Default)]
pub struct PluginManager{
    pub plugins: Vec<Box<dyn Plugin>>,
    pub libraries: Vec<Library>
}

const LIB_LOCATION: &str = "/usr/local/lib/net-gazer";
const ENTRY_POINT: &[u8] = b"net_gazer_plugin_new";

type PluginCreate = unsafe fn() -> *mut dyn Plugin;

impl PluginManager{

    pub fn new(iface:&NetworkInterface, tx:CoreSender) -> Self{

        let mut p_manager = PluginManager::default();

        if let Some(list) = discover(){
            for lib_name in list{
                debug!("about to load library \"{}\"...", lib_name);
                match Library::new(lib_name.clone()){
                    Ok(lib) => {
                        debug!("library \"{}\" loaded! Looking for plugin entry point...", lib_name);
                        unsafe {
                            match lib.get::<Symbol<PluginCreate>>(ENTRY_POINT){
                                Ok(fn_creator) => {
                                    let boxed_raw = fn_creator();
                                    let mut plugin = Box::from_raw(boxed_raw);
                                    debug!("plugin [{}] \"{}\" is found! Initializing...", plugin.get_id(), plugin.get_name());
                                    plugin.on_load(iface, tx.clone());
                                    p_manager.libraries.push(lib);
                                    p_manager.plugins.push(plugin);
                                }
                                Err(e) => error!("Unable to find plugin entry point[{}] in library[{}]. Error:{}", 
                                    std::str::from_utf8_unchecked(ENTRY_POINT), lib_name, e
                                )
                            }
                        }
                    }
                    Err(e) => error!("Failed to load library[{}]. Error:{}",lib_name, e)
                }
            }
        
        }
        p_manager
    }
    
    pub fn len(&self) -> usize{
        self.plugins.len()
    }
    pub fn is_empty(&self) -> bool{
        self.plugins.len() < 1
    }


    pub fn process(&self, pkt:& EthernetPacket){
        //FIXME: parallel
        self.plugins.iter().for_each(|p|p.process(pkt));
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