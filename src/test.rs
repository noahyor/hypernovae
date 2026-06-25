#[test]
fn test_varint_parse() {
    use crate::net::data::parse_varint;
    assert!(parse_varint(&[0x00]).unwrap().1 == 0);
    assert!(parse_varint(&[0x01]).unwrap().1 == 1);
    assert!(parse_varint(&[0x02]).unwrap().1 == 2);
    assert!(parse_varint(&[0x7F]).unwrap().1 == 127);
    assert!(parse_varint(&[0x80, 0x01]).unwrap().1 == 128);
    assert!(parse_varint(&[0xFF, 0x01]).unwrap().1 == 255);
    assert!(parse_varint(&[0xDD, 0xC7, 0x01]).unwrap().1 == 25565);
    assert!(parse_varint(&[0xFF, 0xFF, 0x7F]).unwrap().1 == 2097151);
    assert!(parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0x07]).unwrap().1 == 2147483647);
    assert!(parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]).unwrap().1 == -1);
    assert!(parse_varint(&[0x80, 0x80, 0x80, 0x80, 0x08]).unwrap().1 == -2147483648);
    assert!(parse_varint(&[]).is_err());
    assert!(parse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]).is_err());
    assert!(parse_varint(&[0x80, 0x01, 0xAE]).unwrap().1 == 128);
    let temp = parse_varint(&[0x01; 6]).unwrap();
    assert!(temp.0.len() == 5);
    assert!((temp.0)[0] == 0x01);
}

#[test]
fn test_varlong_parse() {
    use crate::net::data::parse_varlong;
    assert!(parse_varlong(&[0x00]).unwrap().1 == 0);
    assert!(parse_varlong(&[0x01]).unwrap().1 == 1);
    assert!(parse_varlong(&[0x02]).unwrap().1 == 2);
    assert!(parse_varlong(&[0x7F]).unwrap().1 == 127);
    assert!(parse_varlong(&[0x80, 0x01]).unwrap().1 == 128);
    assert!(parse_varlong(&[0xFF, 0x01]).unwrap().1 == 255);
    assert!(parse_varlong(&[0xFF, 0xFF, 0xFF, 0xFF, 0x07]).unwrap().1 == 2147483647);
    assert!(
        parse_varlong(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F])
            .unwrap()
            .1
            == 9223372036854775807
    );
    assert!(
        parse_varlong(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F])
            .unwrap()
            .1
            == -1
    );
    assert!(
        parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0xF8, 0xFF, 0xFF, 0xFF, 0xFF, 0x01])
            .unwrap()
            .1
            == -2147483648
    );
    assert!(
        parse_varlong(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01])
            .unwrap()
            .1
            == -9223372036854775808
    );
    assert!(parse_varlong(&[]).is_err());
    assert!(
        parse_varlong(&[
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01
        ])
        .is_err()
    );
    assert!(parse_varlong(&[0x80, 0x01, 0xAE]).unwrap().1 == 128);
    let temp = parse_varlong(&[0x01; 11]).unwrap();
    assert!(temp.0.len() == 10);
    assert!((temp.0)[0] == 0x01);
}
