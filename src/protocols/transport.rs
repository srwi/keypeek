//! Raw hardware transport abstractions for USB/BLE HID devices.

use super::DeviceError;
use std::time::Duration;

#[cfg(test)]
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// Port interface for communicating with raw HID device endpoints.
///
/// Implemented by native desktop USB HID adapters (via `hidapi`), browser WebHID
/// adapters in WebAssembly builds, and in-memory mock transports for unit testing.
pub trait RawHidTransport: Send {
    /// Writes raw output report bytes to the HID device.
    fn write_output_report(&mut self, data: &[u8]) -> Result<(), DeviceError>;

    /// Reads an incoming input report with the specified timeout.
    ///
    /// Returns:
    /// - `Ok(Some(bytes))` if a packet was received.
    /// - `Ok(None)` if no packet was received within the timeout.
    /// - `Err(DeviceError)` if a transport I/O error occurred.
    fn read_input_report(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, DeviceError>;
}

/// In-memory mock implementation of [`RawHidTransport`] for unit testing.
///
/// Supports queuing incoming packets to be returned by `read_input_report`,
/// and recording outgoing packets written by `write_output_report`.
/// Cloning a `MockHidTransport` shares the same underlying queues, allowing
/// tests to inspect written reports or push incoming reports while the transport
/// is passed to a background thread or protocol instance.
#[cfg(test)]
#[derive(Default, Debug, Clone)]
pub struct MockHidTransport {
    incoming: Arc<Mutex<VecDeque<Vec<u8>>>>,
    written: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[cfg(test)]
impl MockHidTransport {
    /// Creates a new, empty mock transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueues an incoming raw packet that will be returned by [`RawHidTransport::read_input_report`].
    pub fn push_incoming(&self, packet: Vec<u8>) {
        self.incoming.lock().unwrap().push_back(packet);
    }

    /// Returns a snapshot of all raw packets written via [`RawHidTransport::write_output_report`].
    pub fn written_packets(&self) -> Vec<Vec<u8>> {
        self.written.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl RawHidTransport for MockHidTransport {
    fn write_output_report(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        self.written.lock().unwrap().push(data.to_vec());
        Ok(())
    }

    fn read_input_report(&mut self, _timeout: Duration) -> Result<Option<Vec<u8>>, DeviceError> {
        Ok(self.incoming.lock().unwrap().pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::{pump_hid_reader, DeviceEvent};
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn mock_transport_write_and_read() {
        let mut transport = MockHidTransport::new();
        transport.push_incoming(vec![1, 2, 3]);

        let read = transport
            .read_input_report(Duration::from_millis(10))
            .unwrap();
        assert_eq!(read, Some(vec![1, 2, 3]));

        let empty = transport
            .read_input_report(Duration::from_millis(10))
            .unwrap();
        assert_eq!(empty, None);

        transport.write_output_report(&[0xC0, 0xA1]).unwrap();
        assert_eq!(transport.written_packets(), vec![vec![0xC0, 0xA1]]);
    }

    #[test]
    fn mock_transport_shares_state_on_clone() {
        let transport = MockHidTransport::new();
        let mut clone = transport.clone();

        clone.write_output_report(&[42]).unwrap();
        assert_eq!(transport.written_packets(), vec![vec![42]]);

        transport.push_incoming(vec![99]);
        assert_eq!(
            clone.read_input_report(Duration::from_millis(10)).unwrap(),
            Some(vec![99])
        );
    }

    #[test]
    fn mock_transport_pumps_events_through_reader() {
        let transport = MockHidTransport::new();
        let mut reader_transport = transport.clone();

        // Queue a key event packet: [0xF1, row=1, col=2, pressed=1]
        transport.push_incoming(vec![0xF1, 1, 2, 1]);

        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            pump_hid_reader(
                || {
                    match reader_transport.read_input_report(Duration::from_millis(10)) {
                        Ok(Some(bytes)) => Ok(Some(bytes)),
                        Ok(None) => Err("disconnect".to_string()), // trigger exit after first read
                        Err(e) => Err(e.to_string()),
                    }
                },
                tx,
                "Disconnected",
            );
        });

        let event = rx.recv().unwrap();
        assert_eq!(
            event,
            DeviceEvent::KeyPressed {
                row: 1,
                col: 2,
                pressed: true
            }
        );

        let disconnect = rx.recv().unwrap();
        assert_eq!(
            disconnect,
            DeviceEvent::Disconnected("Disconnected".to_string())
        );
        let _ = handle.join();
    }
}
