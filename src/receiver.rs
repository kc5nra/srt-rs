use std::io::{Error, ErrorKind};
use std::time::Duration;
use std::net::SocketAddr;

use socket::SrtSocket;
use packet::{AckControlInfo, ControlTypes, Packet};
use bytes::BytesMut;
use futures::prelude::*;
use futures_timer::Interval;

struct LossListEntry {
    seq_num: i32,
    feedback_time: i32,

    // the nubmer of times this entry has been fed back into NAK
    k: i32,
}

pub struct Receiver {
    remote: SocketAddr,

    /// https://tools.ietf.org/html/draft-gg-udt-03#page-12
    /// Receiver's Loss List: It is a list of tuples whose values include:
    /// the sequence numbers of detected lost data packets, the latest
    /// feedback time of each tuple, and a parameter k that is the number
    /// of times each one has been fed back in NAK. Values are stored in
    /// the increasing order of packet sequence numbers.
    loss_list: Vec<LossListEntry>,

    /// https://tools.ietf.org/html/draft-gg-udt-03#page-12
    /// ACK History Window: A circular array of each sent ACK and the time
    /// it is sent out. The most recent value will overwrite the oldest
    /// one if no more free space in the array.
    ack_history_window: Vec<(i32, i32)>,

    /// https://tools.ietf.org/html/draft-gg-udt-03#page-12
    /// PKT History Window: A circular array that records the arrival time
    /// of each data packet.
    packet_history_window: Vec<(i32, i32)>,

    /// Tells the receiver to ACK the sender
    ack_timer: Interval,

    /// the highest received packet sequence number
    lrsn: i32,

    next_ack: i32,
}

impl Receiver {
    pub fn new(sock: SrtSocket, remote: SocketAddr) -> Receiver {
        Receiver {
            sock,
            remote,
            loss_list: Vec::new(),
            ack_history_window: Vec::new(),
            packet_history_window: Vec::new(),
            // TODO: what's the actual ACK timeout?
            ack_timer: Interval::new(Duration::from_secs(1)),
            lrsn: 0,
            next_ack: 0,
        }
    }
}

impl Stream for Receiver {
    type Item = BytesMut;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<BytesMut>, Error> {
        loop {
            // https://tools.ietf.org/html/draft-gg-udt-03#page-12
            // Query the system time to check if ACK, NAK, or EXP timer has
            // expired. If there is any, process the event (as described below
            // in this section) and reset the associated time variables. For
            // ACK, also check the ACK packet interval.

            if let Async::Ready(_) = self.ack_timer.poll()? {
                // Send an ACK packet
                let ack = Packet::Control {
                    timestamp: self.sock.get_timestamp(),
                    dest_sockid: 0, // TODO: this should be better
                    control_type: ControlTypes::Ack(self.next_ack, AckControlInfo::new(self.lrsn)),
                };
                self.next_ack += 1;

                self.sock.queue_sender.send((ack, self.remote)).unwrap()
            }

            // wait for a packet
            // TODO: have some sort of set timeout and store EXPCount
            let (pack, addr) = match try_ready!(self.sock.poll()) {
                Some(p) => p,
                None => panic!(), // TODO: is this panic safe?
            };

            // depending on the packet type, handle it
            match pack {
                Packet::Control {
                    timestamp,
                    dest_sockid,
                    ref control_type,
                } => {
                    // handle the control packet

                    match control_type {
                        &ControlTypes::Ack(seq_num, info) => unimplemented!(),
                        &ControlTypes::Ack2(seq_num) => unimplemented!(),
                        &ControlTypes::DropRequest(to_drop, info) => unimplemented!(),
                        &ControlTypes::Handshake(info) => {
                            // just send it back
                            self.sock
                                .queue_sender
                                .send((pack.clone(), self.remote))
                                .unwrap();
                        }
                        &ControlTypes::KeepAlive => unimplemented!(),
                        &ControlTypes::Nak(ref info) => unimplemented!(),
                        &ControlTypes::Shutdown => unimplemented!(),
                    }
                }
                Packet::Data {
                    seq_number,
                    message_loc,
                    in_order_delivery,
                    message_number,
                    timestamp,
                    dest_sockid,
                    payload,
                } => {
                    self.lrsn = seq_number;
                }
            }
        }
    }
}