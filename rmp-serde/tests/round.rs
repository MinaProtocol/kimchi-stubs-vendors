use rmp_serde::config::{DefaultConfig, SerializerConfig};
use rmp_serde::decode::ReadReader;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::io::Cursor;

#[test]
#[ignore]
fn issue_250() {
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    #[serde(tag = "type", content = "payload")]
    pub enum Example {
        Unit1,
        Unit2,
        HasValue { x: u32 },
        TupleWithValue(u32, u32),
        InnerValue(SomeInnerValue),
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    pub struct SomeInnerValue {
        pub a: u32,
        pub b: String,
    }

    let v = rmp_serde::to_vec(&Example::HasValue { x: 3 }).unwrap();

    // Fails unwrap with Syntax("invalid type: sequence,
    // expected struct variant Example::HasValue")
    let ex: Example = rmp_serde::from_slice(&v).unwrap();
    assert_eq!(Example::HasValue { x: 3 }, ex);

    let v = rmp_serde::to_vec(&Example::Unit1).unwrap();

    // Fails unwrap with Syntax("invalid length 1,
    // expected adjacently tagged enum Example")
    let ex: Example = rmp_serde::from_slice(&v).unwrap();
    assert_eq!(Example::Unit1, ex);
}

#[test]
fn round_trip_option() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Foo {
        v: Option<Vec<u8>>,
    }

    let expected = Foo { v: None };

    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(Cursor::new(&buf[..]));

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_trip_nested_option() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Struct {
        f1: Option<Option<u32>>,
        f2: Option<Option<u32>>,
    }

    let expected = Struct {
        f1: Some(Some(13)),
        f2: None,
    };

    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(Cursor::new(&buf[..]));

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_trip_optional_enum() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub enum SimpleEnum {
        Variant,
    }
    let expected = Some(SimpleEnum::Variant);

    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(Cursor::new(&buf[..]));
    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_trip_cow() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Foo<'a> {
        v: Cow<'a, [u8]>,
    }

    let expected = Foo {
        v: Cow::Borrowed(&[]),
    };

    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(Cursor::new(&buf[..]));

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_trip_option_cow() {
    use serde::Serialize;
    use std::borrow::Cow;
    use std::io::Cursor;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Foo<'a> {
        v: Option<Cow<'a, [u8]>>,
    }

    let expected = Foo { v: None };

    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(Cursor::new(&buf[..]));

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_struct_like_enum() {
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        A { data: u32 },
    }

    let expected = Enum::A { data: 42 };
    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(&buf[..]);

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_struct_like_enum_with_struct_map() {
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        A { data: u32 },
    }

    let expected = Enum::A { data: 42 };
    let mut buf = Vec::new();
    expected
        .serialize(&mut Serializer::new(&mut buf).with_struct_map())
        .unwrap();

    let mut de = Deserializer::new(&buf[..]);

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_struct_like_enum_with_struct_tuple() {
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        A { data: u32 },
    }

    let expected = Enum::A { data: 42 };
    let mut buf = Vec::new();
    expected
        .serialize(&mut Serializer::new(&mut buf).with_struct_tuple())
        .unwrap();

    let mut de = Deserializer::new(&buf[..]);

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_enum_with_newtype_struct() {
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Newtype(String);

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Enum {
        A(Newtype),
    }

    let expected = Enum::A(Newtype("le message".into()));
    let mut buf = Vec::new();
    expected.serialize(&mut Serializer::new(&mut buf)).unwrap();

    let mut de = Deserializer::new(&buf[..]);

    assert_eq!(expected, Deserialize::deserialize(&mut de).unwrap());
}

#[test]
fn round_trip_untagged_enum_with_enum_associated_data() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum Foo {
        A(Bar),
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Bar {
        B,
        C(String),
        D(u64, u64, u64),
        E { f1: String },
    }

    let data1_1 = Foo::A(Bar::B);
    let bytes_1 = rmp_serde::to_vec(&data1_1).unwrap();
    let data1_2 = rmp_serde::from_slice(&bytes_1).unwrap();
    assert_eq!(data1_1, data1_2);

    let data2_1 = Foo::A(Bar::C("Hello".into()));
    let bytes_2 = rmp_serde::to_vec(&data2_1).unwrap();
    let data2_2 = rmp_serde::from_slice(&bytes_2).unwrap();
    assert_eq!(data2_1, data2_2);

    let data3_1 = Foo::A(Bar::D(1, 2, 3));
    let bytes_3 = rmp_serde::to_vec(&data3_1).unwrap();
    let data3_2 = rmp_serde::from_slice(&bytes_3).unwrap();
    assert_eq!(data3_1, data3_2);

    let data4_1 = Foo::A(Bar::E { f1: "Hello".into() });
    let bytes_4 = rmp_serde::to_vec(&data4_1).unwrap();
    let data4_2 = rmp_serde::from_slice(&bytes_4).unwrap();
    assert_eq!(data4_1, data4_2);
}

// Checks whether deserialization and serialization can both work with structs as maps
#[test]
fn round_struct_as_map() {
    use rmp_serde::decode::from_slice;
    use rmp_serde::to_vec_named;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Dog1 {
        name: String,
        age: u16,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Dog2 {
        age: u16,
        name: String,
    }

    let dog1 = Dog1 {
        name: "Frankie".into(),
        age: 42,
    };

    let serialized: Vec<u8> = to_vec_named(&dog1).unwrap();
    let deserialized: Dog2 = from_slice(&serialized).unwrap();

    let check = Dog1 {
        age: deserialized.age,
        name: deserialized.name,
    };

    assert_eq!(dog1, check);
}

#[test]
fn round_struct_as_map_in_vec() {
    // See: issue #205
    use rmp_serde::decode::from_slice;
    use rmp_serde::to_vec_named;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Dog1 {
        name: String,
        age: u16,
    }
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Dog2 {
        age: u16,
        name: String,
    }

    let dog1 = Dog1 {
        name: "Frankie".into(),
        age: 42,
    };

    let data = vec![dog1];

    let serialized: Vec<u8> = to_vec_named(&data).unwrap();
    let deserialized: Vec<Dog2> = from_slice(&serialized).unwrap();

    let dog2 = &deserialized[0];

    assert_eq!(dog2.name, "Frankie");
    assert_eq!(dog2.age, 42);
}

#[test]
fn round_trip_unit_struct() {
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Message1 {
        data: u8,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Message2;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    enum Messages {
        Message1(Message1),
        Message2(Message2),
    }

    let msg2 = Messages::Message2(Message2);

    // struct-as-tuple
    {
        let serialized: Vec<u8> = rmp_serde::to_vec(&msg2).unwrap();
        let deserialized: Messages = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, msg2);
    }

    // struct-as-map
    {
        let serialized: Vec<u8> = rmp_serde::to_vec_named(&msg2).unwrap();
        let deserialized: Messages = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, msg2);
    }
}

#[test]
#[ignore]
fn round_trip_unit_struct_untagged_enum() {
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct UnitStruct;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct MessageA {
        some_int: i32,
        unit: UnitStruct,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(untagged)]
    enum Messages {
        MessageA(MessageA),
    }

    let msga = Messages::MessageA(MessageA {
        some_int: 32,
        unit: UnitStruct,
    });

    // struct-as-tuple
    {
        let serialized: Vec<u8> = rmp_serde::to_vec(&msga).unwrap();
        let deserialized: Messages = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, msga);
    }

    // struct-as-map
    {
        let serialized: Vec<u8> = rmp_serde::to_vec_named(&msga).unwrap();
        let deserialized: Messages = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, msga);
    }
}

#[test]
fn round_trip_struct_with_flattened_map_field() {
    use std::collections::BTreeMap;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Struct {
        f1: u32,
        // not flattend!
        f2: BTreeMap<String, String>,
        #[serde(flatten)]
        f3: BTreeMap<String, String>,
    }

    let strct = Struct {
        f1: 0,
        f2: {
            let mut map = BTreeMap::new();
            map.insert("german".to_string(), "Hallo Welt!".to_string());
            map
        },
        f3: {
            let mut map = BTreeMap::new();
            map.insert("english".to_string(), "Hello World!".to_string());
            map
        },
    };

    let serialized: Vec<u8> = rmp_serde::to_vec(&strct).unwrap();
    let deserialized: Struct = rmp_serde::from_slice(&serialized).unwrap();
    assert_eq!(deserialized, strct);
}

#[test]
fn round_trip_struct_with_flattened_struct_field() {
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Struct {
        f1: u32,
        // not flattend!
        f2: InnerStruct,
        #[serde(flatten)]
        f3: InnerStruct,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct InnerStruct {
        f4: u32,
        f5: u32,
    }

    let strct = Struct {
        f1: 0,
        f2: InnerStruct { f4: 8, f5: 13 },
        f3: InnerStruct { f4: 21, f5: 34 },
    };

    // struct-as-tuple
    {
        let serialized: Vec<u8> = rmp_serde::to_vec(&strct).unwrap();
        let deserialized: Struct = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, strct);
    }

    // struct-as-map
    {
        let serialized: Vec<u8> = rmp_serde::to_vec_named(&strct).unwrap();
        let deserialized: Struct = rmp_serde::from_slice(&serialized).unwrap();
        assert_eq!(deserialized, strct);
    }
}

// Checks whether deserialization and serialization can both work with enum variants as strings
#[test]
fn round_variant_string() {
    use rmp_serde::decode::from_slice;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    enum Animal1 {
        Dog { breed: String },
        Cat,
        Emu,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    enum Animal2 {
        Emu,
        Dog { breed: String },
        Cat,
    }

    // use helper macro so that we can test many combinations at once. Needs to be a macro to deal
    // with the serializer owning a reference to the Vec.
    macro_rules! do_test {
        ($ser:expr) => {
            {
                let animal1 = Animal1::Dog { breed: "Pitbull".to_owned() };
                let expected = Animal2::Dog { breed: "Pitbull".to_owned() };
                let mut buf = Vec::new();
                animal1.serialize(&mut $ser(&mut buf)).unwrap();

                let deserialized: Animal2 = from_slice(&buf).unwrap();
                assert_eq!(deserialized, expected);
            }
        }
    }

    do_test!(Serializer::new);
    do_test!(|b| Serializer::new(b).with_struct_map());
    do_test!(|b| Serializer::new(b).with_struct_tuple());
    do_test!(|b| Serializer::new(b).with_struct_map());
    do_test!(|b| Serializer::new(b).with_struct_tuple());
    do_test!(|b| {
        Serializer::new(b)
            .with_struct_tuple()
            .with_struct_map()
            .with_struct_tuple()
            .with_struct_map()
    });
}

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn roundtrip_ipv4addr() {
    assert_roundtrips(Ipv4Addr::new(127, 0, 0, 1));
}

#[test]
fn roundtrip_ipv6addr() {
    assert_roundtrips(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8));
}

#[test]
fn roundtrip_ipaddr_ipv4addr() {
    assert_roundtrips(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}

#[test]
fn roundtrip_ipaddr_ipv6addr() {
    assert_roundtrips(IpAddr::V6(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8)));
}

#[test]
fn roundtrip_result_ipv4addr() {
    let val: Result<Ipv4Addr, ()> = Ok(Ipv4Addr::new(127, 0, 0, 1));
    assert_roundtrips(val);
}

#[test]
fn roundtrip_result_num() {
    assert_roundtrips(Ok::<u32, u32>(42));
    assert_roundtrips(Err::<(), _>(222));
}

#[test]
fn roundtrip_simple_enum() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    enum SimpleEnum {
        V1(u32),
        V2(String),
    }

    assert_roundtrips(SimpleEnum::V1(42));
    assert_roundtrips(SimpleEnum::V2("hello".into()));
}

#[test]
fn roundtrip_some() {
    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct Wrapper<T>(T);

    assert_roundtrips(Some(99));
    assert_roundtrips(Wrapper(Some(99)));
    assert_roundtrips(Some(Wrapper(99)));
    assert_roundtrips(Some("hi".to_string()));
}

/// Some types don't fully consume their input SeqAccess, leading to incorrect
/// deserializes.
///
/// https://github.com/3Hren/msgpack-rust/issues/287
#[test]
fn checked_seq_access_len() {
    #[derive(Serialize)]
    struct Input {
        a: [&'static str; 4],
        d: &'static str,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    struct Output {
        a: [String; 2],
        c: String,
    }

    let mut buffer = Vec::new();
    let mut serializer = Serializer::new(&mut buffer)
        .with_binary()
        .with_struct_map();

    // The bug is basically that Output will successfully deserialize from input
    // because the [String; 0] deserializer doesn't drain the SeqAccess, and
    // the two fields it leaves behind can then be deserialized into `v`

    let data = Input {
        a: ["b", "b", "c", "c"],
        d: "d",
    };

    data.serialize(&mut serializer)
        .expect("failed to serialize");

    let mut deserializer = rmp_serde::Deserializer::new(
        Cursor::new(&buffer)
    ).with_binary();

    Output::deserialize(&mut deserializer)
        .expect_err("Input round tripped into Output; this shouldn't happen");
}

#[test]
fn array_from_bytes() {
    let orig = [1u8, 128, 255];
    let v = rmp_serde::to_vec(orig.as_slice()).unwrap();
    let arr: [u8; 3] = rmp_serde::from_slice(&v).unwrap();
    assert_eq!(arr, orig);
    let tup: (u8, u8, u8) = rmp_serde::from_slice(&v).unwrap();
    assert_eq!(tup, (1, 128, 255));
}

#[test]
fn i128_from_integers() {
    let v = rmp_serde::to_vec([0, 1i8, -12i8, 119].as_slice()).unwrap();
    let arr: [i128; 4] = rmp_serde::from_slice(&v).unwrap();
    assert_eq!(arr, [0, 1i128, -12, 119]);
}

#[ignore]
#[test]
fn roundtrip_some_failures() {
    // FIXME
    assert_roundtrips(Some(None::<()>));
}

#[cfg(test)]
#[track_caller]
fn assert_roundtrips<T: PartialEq + std::fmt::Debug + Serialize + for<'a> Deserialize<'a>>(val: T) {
    assert_roundtrips_config(&val, "default", |s| s, |d| d);
    assert_roundtrips_config(&val, ".with_struct_map()", |s| s.with_struct_map(), |d| d);
    assert_roundtrips_config(
        &val,
        ".with_struct_map()",
        |s| s.with_struct_map(),
        |d| d,
    );
    assert_roundtrips_config(
        &val,
        ".with_human_readable()",
        |s| s.with_human_readable(),
        |d| d.with_human_readable(),
    );
    assert_roundtrips_config(
        &val,
        ".with_human_readable().with_struct_map()",
        |s| s.with_human_readable().with_struct_map(),
        |d| d.with_human_readable(),
    );
    assert_roundtrips_config(
        &val,
        ".with_human_readable()",
        |s| s.with_human_readable(),
        |d| d.with_human_readable(),
    );
    assert_roundtrips_config(
        &val,
        ".with_human_readable().with_struct_map()",
        |s| {
            s.with_human_readable()
                .with_struct_map()
        },
        |d| d.with_human_readable(),
    );
}

#[cfg(test)]
#[track_caller]
fn assert_roundtrips_config<T, CSF, SC, CDF, DC>(
    val: &T,
    desc: &str,
    config_serializer: CSF,
    config_deserializer: CDF,
) where
    T: PartialEq + std::fmt::Debug + Serialize + for<'a> Deserialize<'a>,
    CSF: FnOnce(Serializer<Vec<u8>, DefaultConfig>) -> Serializer<Vec<u8>, SC>,
    SC: SerializerConfig,
    CDF: FnOnce(
        Deserializer<ReadReader<&[u8]>, DefaultConfig>,
    ) -> Deserializer<ReadReader<&[u8]>, DC>,
    DC: SerializerConfig,
{
    let mut serializer = config_serializer(Serializer::new(Vec::new()));
    if let Err(e) = val.serialize(&mut serializer) {
        panic!(
            "Failed to serialize: {}\nConfig: {}\nValue: {:?}\n",
            e, desc, val
        );
    }
    let serialized = serializer.into_inner();

    let mut deserializer = config_deserializer(Deserializer::new(serialized.as_slice()));
    let val2: T = match T::deserialize(&mut deserializer) {
        Ok(t) => t,
        Err(e) => {
            panic!(
                "Does not deserialize: {}\nConfig: {}\nSerialized {:?}\nGot {:?}\n",
                e,
                desc,
                val,
                rmpv::decode::value::read_value(&mut serialized.as_slice())
                    .expect("rmp didn't serialize correctly at all")
            );
        }
    };

    assert_eq!(val, &val2, "Config: {}", desc);
}
