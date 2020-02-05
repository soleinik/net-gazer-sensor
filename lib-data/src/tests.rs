#[test]
fn another() {
    let orig:std::net::Ipv4Addr = "192.168.1.1".parse().unwrap();

    let addr_as_int: u32 = orig.into();
    let actual = std::net::Ipv4Addr::from(addr_as_int);

    assert_eq!(orig, actual);

    println!("==================>Make this test fail");
}


