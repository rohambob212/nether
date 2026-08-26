use crate::settings::NetherSettings;
use serde_json::json;

/// Generate the Xray-core config implementing Hiddify-style Iran split
/// routing: ads blocked, Iranian sites and private IPs direct, everything
/// else forwarded through the Aether tunnel's local SOCKS5.
///
/// The first outbound is Xray's default, so unknown traffic goes to the
/// tunnel — fail closed, not open.
pub fn gen_config(settings: &NetherSettings) -> String {
    let cfg = json!({
        "log": { "loglevel": "warning" },
        "inbounds": [{
            "listen": settings.socks_host,
            "port": settings.xray_socks_port,
            "protocol": "socks",
            "settings": { "udp": true },
            "sniffing": { "enabled": true, "destOverride": ["http", "tls", "quic"] }
        }],
        "outbounds": [
            {
                "tag": "proxy",
                "protocol": "socks",
                "settings": { "servers": [{ "address": settings.socks_host, "port": settings.socks_port }] }
            },
            { "tag": "direct", "protocol": "freedom" },
            { "tag": "block", "protocol": "blackhole" }
        ],
        "routing": {
            "domainStrategy": "IPIfNonMatch",
            "rules": [
                { "outboundTag": "block", "domain": ["geosite:category-ads-all"] },
                { "outboundTag": "direct", "domain": ["geosite:category-ir"] },
                { "outboundTag": "direct", "ip": ["geoip:ir", "geoip:private"] }
            ]
        }
    });
    serde_json::to_string_pretty(&cfg).expect("xray config serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_routes_foreign_via_tunnel_and_iran_direct() {
        let mut s = NetherSettings::default();
        s.smart_routing = true;
        let cfg: serde_json::Value = serde_json::from_str(&gen_config(&s)).unwrap();

        // Apps connect here...
        assert_eq!(cfg["inbounds"][0]["port"], s.xray_socks_port);
        // ...default outbound is the Aether tunnel (fail closed)...
        assert_eq!(cfg["outbounds"][0]["tag"], "proxy");
        assert_eq!(cfg["outbounds"][0]["settings"]["servers"][0]["port"], s.socks_port);
        // ...and Iran/private traffic bypasses it.
        let rules = cfg["routing"]["rules"].as_array().unwrap();
        assert!(rules.iter().any(|r| r["outboundTag"] == "block"));
        assert!(rules.iter().any(|r| {
            r["outboundTag"] == "direct"
                && r["domain"]
                    .as_array()
                    .is_some_and(|ds| ds.iter().any(|v| v == "geosite:category-ir"))
        }));
        assert!(rules.iter().any(|r| {
            r["outboundTag"] == "direct"
                && r["ip"]
                    .as_array()
                    .is_some_and(|ips| ips.iter().any(|v| v == "geoip:ir"))
        }));
    }
}
