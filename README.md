# IPFW

Redirect TCP traffic from one address to another.

```bash
ipfw 0.0.0.0:8080 127.0.0.1:9000
```

Can be used to redirect IPV6 traffic to application binded on IPv4 addresses.

```bash
ipfw [::]:8080 127.0.0.1:8080 --v6-only
```