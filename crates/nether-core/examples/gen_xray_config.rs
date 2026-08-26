fn main() {
    let mut s = nether_core::NetherSettings::default();
    s.smart_routing = true;
    println!("{}", nether_core::xray::gen_config(&s));
}
