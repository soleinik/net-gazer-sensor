## VPN box with no VPN Server running

# Network diagram

![VPN](vpn-server.png)


# setup
"A" is my VPN box, with no VPN server running... This box is accessible from outside via port forwarding. With VPN, (right or wrong), this box serves as ssh jumping host. This host is accesible from inside the network, via ssh and lxc console.

# question
Should there be any traffic?


# net-gazer setup
1. net-gazer-sensor sniffs traffic from 'eth0' nic and sends reports via 'lo'
2. traceroute plugin deployed to net-gazer-sensor
3. net-gazer-web runs on that(for simplicity) box, on 'lo' nic. There is not db running, but transaction log, that I will replay later into database


# notes
traceroute plugin looks for tcp SYN+SYN/ACK combination. Since server socket does not exists - should be no traffic on that host. I tried to portscan my extenal IP - nothing... 
next plugin to work on - record any ethernet frames. I want to capture port scan 

I will leave it running for 24 hours - let's see what happens.....

# results
Well, there was nothing interesting - log is full with own IP entries...Same result even with VPN server up. I did not try to connect to VPN, but I did try to scan external IP with online tools and with nmap - just own IP is logged. I notied that external scans would trigger log activity. So, something happens, but nothing to... write home about

Negative result is also result. 


Moving to next plugin...