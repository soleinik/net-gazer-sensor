# _net-gazer_ - network connection capture and analysis daemon

# Goals
Design daemon that seats on gateway and passively intercepts traversing traffic, detecting SYN and SYN+ACK handshakes portion. Reads remote IP and attempts to traceroute to remote host, capturing hops. 
Captured data can be:
1. graphed
2. geolocation enriched
3. from elapced time between SYN and SYN-ACK derive network performance
4. capturing SACK (tcp retransmits) - derive network quality
5. many other things...




# To build
```
$ cargo build 
```


## To run (cli help)
root is needed to run (./.cargo/config)
```
$ cargo run [-- --help]

$ cargo run -- -i eth0

```


### Configuration files search order
```
./etc/net-gazer/net-gazer.toml
/usr/local/etc/net-gazer/net-gazer.toml
/etc/net-gazer/net-gazer.toml
```

## Help
```
cargo run -- --help

Running `sudo -E target/debug/net-gazer --help`
net-gazer 0.1.0
network connection capture and analysis daemon

USAGE:
    net-gazer [FLAGS] [OPTIONS]

FLAGS:
    -h, --help         Prints help information
    -V, --version      Prints version information
    -v, --verbosity    Verbose mode (-v(info), -vv(debug), -vvv(trace), etc.)

OPTIONS:
    -c, --config <config-path>    configuration file [env: NG_CONFIG=]
    -i, --iface <iface>           target network interface [env: NG_IFACE=]
