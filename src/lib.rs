// Copyright 2023 宋昊文
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fmt, str::FromStr};

pub struct Origin<'a> {
    pub user_id: &'a [u8],
    pub session_id: &'a [u8],
    pub session_version: &'a [u8],
    pub network_type: &'a [u8],
    pub address_type: &'a [u8],
    pub unicast_address: &'a [u8],
}

impl<'a> fmt::Debug for Origin<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Origin")
            .field("user_id", &String::from_utf8_lossy(&self.user_id))
            .field("session_id", &String::from_utf8_lossy(&self.session_id))
            .field(
                "session_version",
                &String::from_utf8_lossy(&self.session_version),
            )
            .field("network_type", &String::from_utf8_lossy(&self.network_type))
            .field("address_type", &String::from_utf8_lossy(&self.address_type))
            .field(
                "unicast_address",
                &String::from_utf8_lossy(&self.unicast_address),
            )
            .finish()
    }
}

pub struct ConnectionData<'a> {
    pub network_type: &'a [u8],
    pub address_type: &'a [u8],
    pub connection_address: &'a [u8],
}

impl<'a> fmt::Debug for ConnectionData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionData")
            .field("network_type", &String::from_utf8_lossy(&self.network_type))
            .field("address_type", &String::from_utf8_lossy(&self.address_type))
            .field(
                "connection_address",
                &String::from_utf8_lossy(&self.connection_address),
            )
            .finish()
    }
}

pub struct Media<'a> {
    pub media_type: &'a [u8],
    pub port: u16,
    pub number_of_ports: i32,
    pub protocol: &'a [u8],
    pub formats: Vec<&'a [u8]>,
    pub connection: Option<ConnectionData<'a>>,
    pub attributes: Vec<&'a [u8]>,
}

impl<'a> fmt::Debug for Media<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Media");
        debug_struct
            .field("media_type", &String::from_utf8_lossy(&self.media_type))
            .field("port", &self.port)
            .field("number_of_ports", &self.number_of_ports)
            .field("protocol", &String::from_utf8_lossy(&self.protocol));
        for format in &self.formats {
            debug_struct.field("format", &String::from_utf8_lossy(format));
        }
        if let Some(connection) = &self.connection {
            debug_struct.field("connection", connection);
        }
        for attribute in &self.attributes {
            debug_struct.field("attribute", &String::from_utf8_lossy(attribute));
        }
        debug_struct.finish()
    }
}

pub struct Sdp<'a> {
    pub version: &'a [u8],
    pub origin: Origin<'a>,
    pub session_name: &'a [u8],
    pub connection: Option<ConnectionData<'a>>,
    pub session_start_time: u64,
    pub session_end_time: u64,
    pub attributes: Vec<&'a [u8]>,
    pub medias: Vec<Media<'a>>,
}

#[derive(Debug)]
enum Phase {
    Begin,
    Set,
    Reading,
    SkippingError,
}

#[derive(Debug)]
enum Section<'a> {
    Main,
    Time,
    Media(
        Option<&'a [u8]>,
        Option<(u16, i32)>,
        Option<&'a [u8]>,
        Option<Vec<&'a [u8]>>,
        Option<ConnectionData<'a>>,
        Option<Vec<&'a [u8]>>,
    ),
}

#[derive(Debug)]
enum Operator<'a> {
    None,

    V,
    O(
        Option<&'a [u8]>,
        Option<&'a [u8]>,
        Option<&'a [u8]>,
        Option<&'a [u8]>,
        Option<&'a [u8]>,
    ),
    S,
    I,
    U,
    E,
    P,
    C(Option<&'a [u8]>, Option<&'a [u8]>),
    B,

    T(Option<&'a [u8]>),
    R,

    Z,
    K,
    A,

    M,
}

impl<'a> Operator<'a> {
    fn get_order(&self) -> i32 {
        match self {
            Operator::None => 0,
            Operator::V => 1,
            Operator::O(_, _, _, _, _) => 2,
            Operator::S => 3,
            Operator::I => 4,
            Operator::U => 5,
            Operator::E => 6,
            Operator::P => 7,
            Operator::C(_, _) => 8,
            Operator::B => 9,
            Operator::T(_) => 10,
            Operator::R => 11,
            Operator::Z => 12,
            Operator::K => 13,
            Operator::A => 14,
            Operator::M => 15,
        }
    }
}

pub trait AsSDP<'a> {
    type Target;
    fn as_sdp(&'a self) -> Option<Self::Target>;
}

impl<'a> AsSDP<'a> for [u8] {
    type Target = Sdp<'a>;
    fn as_sdp(&self) -> Option<Sdp> {
        let mut phase = Phase::Begin;
        let mut section = Section::Main;
        let mut op: Operator = Operator::None;

        let mut slice_start: Option<usize> = None;

        let mut version: Option<&[u8]> = None;
        let mut origin: Option<Origin> = None;
        let mut session_name: Option<&[u8]> = None;
        let mut connection: Option<ConnectionData> = None;

        let mut session_start_time: u64 = 0;
        let mut session_end_time: u64 = 0;

        let mut medias: Vec<Media> = Vec::new();

        let mut attributes: Vec<&[u8]> = Vec::new();

        let mut i = 0;

        while i < self.len() {
            let b = self[i];

            match &phase {
                Phase::Begin => {
                    if b == b'\r' || b == b'\n' {
                        i = i + 1;
                        continue;
                    } else {
                        let mut next_op: Operator = Operator::None;

                        if b == b'v' {
                            next_op = Operator::V;
                        } else if b == b'o' {
                            next_op = Operator::O(None, None, None, None, None);
                        } else if b == b's' {
                            next_op = Operator::S;
                        } else if b == b'i' {
                            next_op = Operator::I;
                        } else if b == b'u' {
                            next_op = Operator::U;
                        } else if b == b'e' {
                            next_op = Operator::E;
                        } else if b == b'p' {
                            next_op = Operator::P;
                        } else if b == b'c' {
                            next_op = Operator::C(None, None);
                        } else if b == b'b' {
                            next_op = Operator::B;
                        } else if b == b't' {
                            next_op = Operator::T(None);
                        } else if b == b'r' {
                            next_op = Operator::R;
                        } else if b == b'z' {
                            next_op = Operator::Z;
                        } else if b == b'k' {
                            next_op = Operator::K;
                        } else if b == b'a' {
                            next_op = Operator::A;
                        } else if b == b'm' {
                            next_op = Operator::M;
                        }

                        match &section {
                            Section::Main => {
                                if op.get_order() < next_op.get_order() {
                                    op = next_op;
                                    phase = Phase::Set;
                                } else if let (Operator::A, Operator::A) = (&op, &next_op) {
                                    op = next_op;
                                    phase = Phase::Set;
                                } else {
                                    phase = Phase::SkippingError;
                                }
                            }

                            Section::Time => {
                                if next_op.get_order() >= Operator::T(None).get_order() {
                                    op = next_op;
                                    phase = Phase::Set;
                                } else {
                                    phase = Phase::SkippingError;
                                }
                            }

                            Section::Media(_, _, _, _, _, _) => {
                                if op.get_order() < next_op.get_order() {
                                    match next_op {
                                        Operator::I
                                        | Operator::C(_, _)
                                        | Operator::B
                                        | Operator::K
                                        | Operator::A
                                        | Operator::M => {
                                            op = next_op;
                                            phase = Phase::Set;
                                        }
                                        _ => {
                                            phase = Phase::SkippingError;
                                        }
                                    }
                                } else if let (Operator::A, Operator::A) = (&op, &next_op) {
                                    op = next_op;
                                    phase = Phase::Set;
                                } else {
                                    phase = Phase::SkippingError;
                                }
                            }
                        }
                    }
                }

                Phase::Reading => {
                    if b == b'\r' || b == b'\n' {
                        match &mut section {
                            Section::Main => match &op {
                                Operator::V => {
                                    if let Some(slice_start) = slice_start {
                                        version.replace(&self[slice_start..i]);
                                    }
                                    slice_start = None;
                                }

                                Operator::O(
                                    user_id,
                                    session_id,
                                    session_version,
                                    network_type,
                                    address_type,
                                ) => {
                                    match (
                                        user_id,
                                        session_id,
                                        session_version,
                                        network_type,
                                        address_type,
                                    ) {
                                        (
                                            Some(user_id),
                                            Some(session_id),
                                            Some(session_version),
                                            Some(network_type),
                                            Some(address_type),
                                        ) => {
                                            if let Some(slice_start) = slice_start {
                                                let slice = &self[slice_start..i];
                                                origin = Some(Origin {
                                                    user_id,
                                                    session_id,
                                                    session_version,
                                                    network_type,
                                                    address_type,
                                                    unicast_address: slice,
                                                })
                                            }
                                            slice_start = None;
                                        }

                                        _ => {
                                            println!(
                                                "Incomplete originator and session identifier",
                                            );
                                            return None;
                                        }
                                    }
                                }

                                Operator::S => {
                                    if let Some(slice_start) = slice_start {
                                        session_name.replace(&self[slice_start..i]);
                                    }
                                    slice_start = None;
                                }

                                Operator::I | Operator::U | Operator::E | Operator::P => {}

                                Operator::C(network_type, address_type) => {
                                    match (network_type, address_type) {
                                        (Some(network_type), Some(address_type)) => {
                                            if let Some(slice_start) = slice_start {
                                                let slice = &self[slice_start..i];
                                                connection = Some(ConnectionData {
                                                    network_type,
                                                    address_type,
                                                    connection_address: slice,
                                                });
                                            }
                                            slice_start = None;
                                        }

                                        _ => {
                                            println!("Incomplete connection information",);
                                            return None;
                                        }
                                    }
                                }

                                Operator::B | Operator::Z | Operator::K => {}

                                Operator::A => {
                                    if let Some(slice_start) = slice_start {
                                        attributes.push(&self[slice_start..i]);
                                    }
                                    slice_start = None;
                                }

                                _ => {
                                    println!("Unknown description in main section");
                                    return None;
                                }
                            },

                            Section::Time => match &op {
                                Operator::T(start_time) => {
                                    if let Some(start_time) = start_time {
                                        if start_time == b"0" {
                                            session_start_time = u64::MIN;
                                        } else {
                                            if let Ok(t) = start_time.to_int::<u64>() {
                                                session_start_time = t - 2208988800;
                                            } else {
                                                println!("Bad time description format",);
                                                return None;
                                            }
                                        }
                                        if let Some(slice_start) = slice_start {
                                            let slice = &self[slice_start..i];
                                            if slice == b"0" {
                                                session_end_time = u64::MAX;
                                            } else {
                                                if let Ok(t) = slice.to_int::<u64>() {
                                                    session_end_time = t - 2208988800;
                                                } else {
                                                    println!("Bad time description format",);
                                                    return None;
                                                }
                                            }
                                        } else {
                                            println!("Bad time description format");
                                            return None;
                                        }
                                        slice_start = None;
                                    } else {
                                        println!("Bad time description format");
                                        return None;
                                    }
                                }

                                Operator::R => {}

                                // Sometimes a= comes after t=
                                Operator::A => {
                                    if let Some(slice_start) = slice_start {
                                        attributes.push(&self[slice_start..i]);
                                    }
                                    slice_start = None;
                                }

                                _ => {
                                    println!("Unknown description in time section");
                                    return None;
                                }
                            },

                            Section::Media(_, _, _, formats, connection, attributes) => match &op {
                                Operator::None => {
                                    if let Some(slice_start) = slice_start {
                                        if let Some(formats) = formats {
                                            formats.push(&self[slice_start..i]);
                                        } else {
                                            let mut v = Vec::new();
                                            v.push(&self[slice_start..i]);
                                            formats.replace(v);
                                        }
                                    }
                                    slice_start = None;
                                }

                                Operator::I => {}

                                Operator::C(network_type, address_type) => {
                                    match (network_type, address_type) {
                                        (Some(network_type), Some(address_type)) => {
                                            if let Some(slice_start) = slice_start {
                                                let slice = &self[slice_start..i];
                                                *connection = Some(ConnectionData {
                                                    network_type,
                                                    address_type,
                                                    connection_address: slice,
                                                });
                                            }
                                            slice_start = None;
                                        }

                                        _ => {
                                            println!("Incomplete connection information",);
                                            return None;
                                        }
                                    }
                                }

                                Operator::B | Operator::K => {}

                                Operator::A => {
                                    if let Some(slice_start) = slice_start {
                                        if let Some(attributes) = attributes {
                                            attributes.push(&self[slice_start..i]);
                                        } else {
                                            let mut v = Vec::new();
                                            v.push(&self[slice_start..i]);
                                            attributes.replace(v);
                                        }
                                    }
                                    slice_start = None;
                                }

                                _ => {}
                            },
                        }

                        phase = Phase::Begin;
                    } else if b == b' ' {
                        let mut freeform = false;

                        match &mut section {
                            Section::Main => match &mut op {
                                Operator::V => {
                                    println!("Duplicated protocol version");
                                    return None;
                                }

                                Operator::O(
                                    user_id,
                                    session_id,
                                    session_version,
                                    network_type,
                                    address_type,
                                ) => {
                                    match (
                                        &user_id,
                                        &session_id,
                                        &session_version,
                                        &network_type,
                                        &address_type,
                                    ) {
                                        (Some(_), Some(_), Some(_), Some(_), None) => {
                                            if let Some(slice_start) = slice_start {
                                                address_type.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        (Some(_), Some(_), Some(_), None, None) => {
                                            if let Some(slice_start) = slice_start {
                                                network_type.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        (Some(_), Some(_), None, None, None) => {
                                            if let Some(slice_start) = slice_start {
                                                session_version.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        (Some(_), None, None, None, None) => {
                                            if let Some(slice_start) = slice_start {
                                                session_id.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        (None, None, None, None, None) => {
                                            if let Some(slice_start) = slice_start {
                                                user_id.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        _ => {
                                            println!("Bad originator and session identifier",);
                                            return None;
                                        }
                                    }
                                }

                                Operator::S => {
                                    println!("Duplicated session name");
                                    return None;
                                }

                                Operator::I | Operator::U | Operator::E | Operator::P => {}

                                Operator::C(network_type, address_type) => {
                                    match (&network_type, &address_type) {
                                        (Some(_), None) => {
                                            if let Some(slice_start) = slice_start {
                                                address_type.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        (None, None) => {
                                            if let Some(slice_start) = slice_start {
                                                network_type.replace(&self[slice_start..i]);
                                            }
                                            slice_start = None;
                                        }

                                        _ => {
                                            println!("Bad connection information");
                                            return None;
                                        }
                                    }
                                }

                                Operator::B | Operator::Z | Operator::K => {}

                                Operator::A => {
                                    freeform = true;
                                }

                                _ => {
                                    println!("Unknown description in main section");
                                    return None;
                                }
                            },

                            Section::Time => match &mut op {
                                Operator::T(start_time) => {
                                    if let Some(slice_start) = slice_start {
                                        start_time.replace(&self[slice_start..i]);
                                    }
                                    slice_start = None;
                                }

                                Operator::R => {}

                                Operator::A => {
                                    freeform = true;
                                }

                                _ => {
                                    println!("Unknown description in time section");
                                    return None;
                                }
                            },

                            Section::Media(media_type, port_pair, protocol, formats, _, _) => {
                                match &mut op {
                                    Operator::None => {
                                        match (&media_type, &port_pair, &protocol, &formats) {
                                            (Some(_), Some(_), Some(_), _) => {
                                                if let Some(slice_start) = slice_start {
                                                    if let Some(formats) = formats {
                                                        formats.push(&self[slice_start..i]);
                                                    } else {
                                                        let mut v = Vec::new();
                                                        v.push(&self[slice_start..i]);
                                                        formats.replace(v);
                                                    }
                                                }
                                                slice_start = None;
                                            }

                                            (Some(_), Some(_), None, None) => {
                                                if let Some(slice_start) = slice_start {
                                                    protocol.replace(&self[slice_start..i]);
                                                }
                                                slice_start = None;
                                            }

                                            (Some(_), None, None, None) => {
                                                if let Some(slice_start) = slice_start {
                                                    let slice = &self[slice_start..i];
                                                    let mut iter = slice.iter();
                                                    if let Some(idx) = iter.position(|c| *c == b'/')
                                                    {
                                                        if let (Ok(port), Ok(number_of_ports)) = (
                                                            slice[..idx].to_int::<u16>(),
                                                            slice[idx + 1..].to_int::<i32>(),
                                                        ) {
                                                            port_pair
                                                                .replace((port, number_of_ports));
                                                        } else {
                                                            println!("Bad media connection information format");
                                                            return None;
                                                        }
                                                    } else {
                                                        if let Ok(port) = slice.to_int::<u16>() {
                                                            port_pair.replace((port, 1));
                                                        } else {
                                                            println!("Bad media connection information format");
                                                            return None;
                                                        }
                                                    }
                                                }
                                                slice_start = None;
                                            }

                                            (None, None, None, None) => {
                                                if let Some(slice_start) = slice_start {
                                                    media_type.replace(&self[slice_start..i]);
                                                }
                                                slice_start = None;
                                            }

                                            _ => {}
                                        }
                                    }

                                    Operator::I => {}

                                    Operator::C(network_type, address_type) => {
                                        match (&network_type, &address_type) {
                                            (Some(_), None) => {
                                                if let Some(slice_start) = slice_start {
                                                    address_type.replace(&self[slice_start..i]);
                                                }
                                                slice_start = None;
                                            }

                                            (None, None) => {
                                                if let Some(slice_start) = slice_start {
                                                    network_type.replace(&self[slice_start..i]);
                                                }
                                                slice_start = None;
                                            }

                                            _ => {
                                                println!("Bad connection information");
                                                return None;
                                            }
                                        }
                                    }
                                    Operator::B | Operator::K => {}

                                    Operator::A => {
                                        freeform = true;
                                    }

                                    _ => {
                                        println!("Unknown description in media section",);
                                        return None;
                                    }
                                }
                            }
                        }

                        if !freeform {
                            slice_start = None;
                        }
                    } else {
                        if slice_start.is_none() {
                            slice_start = Some(i);
                        }
                    }
                }

                Phase::Set => {
                    if b == b'=' {
                        match &mut op {
                            Operator::T(_) | Operator::R => {
                                section = Section::Time;
                            }

                            Operator::M => {
                                if let Section::Media(
                                    Some(media_type),
                                    Some(port_pair),
                                    Some(protocol),
                                    Some(formats),
                                    connection,
                                    Some(attributes),
                                ) = section
                                {
                                    let (port, number_of_ports) = port_pair;
                                    medias.push(Media {
                                        media_type,
                                        port,
                                        number_of_ports,
                                        protocol,
                                        formats,
                                        connection,
                                        attributes,
                                    });
                                }

                                op = Operator::None;

                                section = Section::Media(None, None, None, None, None, None);
                            }

                            _ => {}
                        }

                        phase = Phase::Reading;
                    } else if b == b'\r' || b == b'\n' {
                        phase = Phase::Begin;
                    } else {
                        phase = Phase::SkippingError;
                    }
                }

                Phase::SkippingError => {
                    if b == b'\r' || b == b'\n' {
                        phase = Phase::Begin;
                    }
                }
            }

            i = i + 1;
        }

        if let Section::Media(
            Some(media_type),
            Some(port_pair),
            Some(protocol),
            Some(formats),
            connection,
            Some(attributes),
        ) = section
        {
            let (port, number_of_ports) = port_pair;
            medias.push(Media {
                media_type,
                port,
                number_of_ports,
                protocol,
                formats,
                connection,
                attributes,
            });
        }

        if let (Some(version), Some(origin), Some(session_name)) = (version, origin, session_name) {
            Some(Sdp {
                version,
                origin,
                session_name,
                connection,
                session_start_time,
                session_end_time,
                attributes,
                medias,
            })
        } else {
            println!("Incomplete sdp");
            None
        }
    }
}

impl<'a> fmt::Debug for Sdp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Sdp");
        debug_struct
            .field("version", &String::from_utf8_lossy(&self.version))
            .field("origin", &self.origin)
            .field("session_name", &String::from_utf8_lossy(&self.session_name))
            .field("connection", &self.connection)
            .field("start_time", &self.session_start_time)
            .field("end_time", &self.session_end_time);
        for attribute in &self.attributes {
            debug_struct.field("attribute", &String::from_utf8_lossy(attribute));
        }
        debug_struct.field("medias", &self.medias).finish()
    }
}

trait ToInt {
    fn to_int<R>(&self) -> Result<R, String>
    where
        R: FromStr;
}

impl ToInt for [u8] {
    fn to_int<R>(&self) -> Result<R, String>
    where
        R: FromStr,
    {
        match std::str::from_utf8(self) {
            Ok(s) => match R::from_str(s) {
                Ok(i) => return Ok(i),
                Err(_) => {
                    return Err(String::from("std::num::ParseIntError"));
                }
            },
            Err(e) => {
                // std::str::Utf8Error
                let s: String = format!("{}", e);
                return Err(s);
            }
        }
    }
}
