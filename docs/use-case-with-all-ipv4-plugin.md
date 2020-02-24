## VPN box with no VPN Server running

# Network diagram

![VPN](vpn-server.png)


# setup
"A" is my VPN box, with no VPN server running... This box is accessible from outside via port forwarding. With VPN, (right or wrong), this box serves as ssh jumping host. This host is accesible from inside the network, via ssh and lxc console.

# question
Should there be any traffic?


# net-gazer setup
1. net-gazer-sensor sniffs traffic from 'eth0' nic and sends reports via 'lo'
2. all-ipv4 plugin deployed to net-gazer-sensor
3. net-gazer-web runs on that(for simplicity) box, on 'lo' nic. There is not db running, but transaction log, that I will replay later into database


# notes
Immediatelly - there is much more activity... I will shutdown my VPN server and see now plugin behaves with scans and etc...
OK, one mistery solved - I'm connected with ssh to that box... so, need to add some filtering to plugin. For now, will use "lxc-attach"...  

# record layout
|proto| pkt_id | src -> dst | pkt_len | ip/tcp flags | ip options



# Igmp/Udp packets

```
[Igmp] 9238 192.168.##.1->224.0.0.1 [28] (empty) []
[Igmp] 0 192.168.##.1->224.0.0.251 [32] SYN [148, 4, 0, 0]
[Igmp] 0 192.168.##.1->224.0.0.2 [32] SYN [148, 4, 0, 0]
[Igmp] 0 192.168.##.1->239.255.255.250 [32] SYN [148, 4, 0, 0]
[Igmp] 9239 192.168.##.1->224.0.0.1 [28] (empty) []
[Igmp] 0 192.168.##.1->239.255.255.250 [32] SYN [148, 4, 0, 0]
[Igmp] 0 192.168.##.1->224.0.0.251 [32] SYN [148, 4, 0, 0]
[Igmp] 0 192.168.##.1->224.0.0.2 [32] SYN [148, 4, 0, 0]
...
[Udp] 0 192.168.##.1->192.168.##.255 [242] SYN []
[Udp] 0 192.168.##.1->192.168.##.255 [235] SYN []

```

# Port scan
https://pentest-tools.com/network-vulnerability-scanning/tcp-port-scanner-online-nmap#

```
[Tcp] 5102 139.162.209.251->192.168.##.A [44] SYN []
[Tcp] 6889 139.162.209.251->192.168.##.A [44] SYN []
[Tcp] 1878 139.162.209.251->192.168.##.A [44] SYN []
```


# nmap scans
```
[Tcp] 31783 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 39805 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 56117 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 38480 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 11320 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 4872 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 41433 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 20941 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 31700 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 36517 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 49071 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 25385 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 58120 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 64086 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 10730 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 10731 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 55201 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 55202 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 18127 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 57586 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 20288 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 54607 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 20709 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 37638 173.151.81.212->192.168.##.A [60] SYN []
[Tcp] 8808 173.151.81.212->192.168.##.A [44] SYN []

# nmap -sU -p 443 
[Udp] 3838 173.151.81.212->192.168.##.A [95] (empty) []

#this one made my box to reply???
[Icmp] 28906 192.168.##.A->173.151.81.212 [123] (empty) []

# nmap -sX -p 443 
[Tcp] 4964 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 10354 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 52524 173.151.81.212->192.168.##.A [44] SYN []

# nmap -sA -p 443 
[Tcp] 3429 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 21904 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 24072 173.151.81.212->192.168.##.A [44] SYN []

# nmap -sW -p 443 
[Tcp] 52877 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 28240 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 46697 173.151.81.212->192.168.##.A [44] SYN []

# nmap -sM -p 443 
[Tcp] 20790 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 64642 173.151.81.212->192.168.##.A [44] SYN []
[Tcp] 51928 173.151.81.212->192.168.##.A [44] SYN []
```



