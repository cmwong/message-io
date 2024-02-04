use message_io::network::{NetEvent, Transport};
use message_io::node::{self};
use bytes::{BufMut, BytesMut};
use integer_encoding::FixedInt;

pub fn main() {
    // test xf client
    let transport = Transport::Xf;
    let remote_addr = "172.16.10.222:9001";

    let (handler, listener) = node::split::<()>();
    let (server_id, local_addr) = handler.network().connect(transport, remote_addr).unwrap();

    // send login to server
    let mut buf = BytesMut::with_capacity(1024);
    buf.put_u8(2); // msgtype
    buf.put_u8(0); // extFlag
    buf.put_u8(0); // extFlag1
    buf.put_u16_le(0x02); // MainCmd
    buf.put_u16_le(0x01); // subCmd
    buf.put(&b"0|123456|true|2023-12-29 00:00:02"[..]);
    let data = &buf.freeze();

    // handler.network().send(server_id, data);

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, established) => {
            if established {
                println!("Connected to server at {} by {}", server_id.addr(), transport);
                println!("Client identified by local port: {}", local_addr.port());
                handler.network().send(server_id, data);
            } else {
                println!("Can not connect to server at {} by {}", remote_addr, transport)
            }
        }
        NetEvent::Accepted(_, _) => unreachable!(), // Only generated when a listener accepts
        NetEvent::Message(_, data) => {
            println!("{:?} {:?} {}", u16::decode_fixed(&data[3..4]), u16::decode_fixed(&data[5..6]), String::from_utf8_lossy(&data[7..]));
        }
        NetEvent::Disconnected(_) => (),
    });
}
