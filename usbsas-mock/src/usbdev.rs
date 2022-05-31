use log::{error, trace};
use std::{env, os::unix::io::RawFd};
use thiserror::Error;
use usbsas_comm::{protoresponse, Comm};
use usbsas_process::UsbsasProcess;
use usbsas_proto as proto;
use usbsas_proto::common::Device as UsbDevice;

#[derive(Error, Debug)]
enum Error {
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Bad Request")]
    BadRequest,
}
type Result<T> = std::result::Result<T, Error>;

protoresponse!(
    CommUsbdev,
    usbdev,
    devices = Devices[ResponseDevices],
    error = Error[ResponseError],
    end = End[ResponseEnd]
);

pub struct MockUsbDev {
    comm: Comm<proto::usbdev::Request>,
    devices: Vec<UsbDevice>,
}

impl MockUsbDev {
    fn new(comm: Comm<proto::usbdev::Request>) -> Result<Self> {
        let mut devices = Vec::new();

        // Fake input device
        if env::var("USBSAS_MOCK_IN_DEV").is_ok() {
            devices.push(UsbDevice {
                busnum: 1,
                devnum: 1, // 1 = INPUT
                vendorid: 1,
                productid: 1,
                manufacturer: "dd".to_string(),
                description: "fake input dev".to_string(),
                serial: "plop".to_string(),
                is_src: true,
                is_dst: false,
            });
        }

        // Fake output device
        if env::var("USBSAS_MOCK_OUT_DEV").is_ok() {
            devices.push(UsbDevice {
                busnum: 1,
                devnum: 2, // 2 = OUTPUT
                vendorid: 1,
                productid: 1,
                manufacturer: "dd".to_string(),
                description: "fake output dev".to_string(),
                serial: "plop".to_string(),
                is_src: false,
                is_dst: true,
            });
        }

        Ok(MockUsbDev { comm, devices })
    }

    fn handle_req_devices(&mut self) -> Result<()> {
        self.comm
            .devices(proto::usbdev::ResponseDevices {
                devices: self.devices.clone(),
            })
            .map_err(|e| e.into())
    }

    fn main_loop(&mut self) -> Result<()> {
        trace!("main loop");
        loop {
            let req: proto::usbdev::Request = self.comm.recv()?;
            let res = match req.msg {
                Some(proto::usbdev::request::Msg::Devices(_)) => self.handle_req_devices(),
                Some(proto::usbdev::request::Msg::End(_)) => {
                    self.comm.end(proto::usbdev::ResponseEnd {})?;
                    break;
                }
                None => Err(Error::BadRequest),
            };
            match res {
                Ok(_) => continue,
                Err(err) => {
                    self.comm.error(proto::usbdev::ResponseError {
                        err: format!("{}", err),
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl UsbsasProcess for MockUsbDev {
    fn spawn(
        read_fd: RawFd,
        write_fd: RawFd,
        _args: Option<Vec<String>>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        MockUsbDev::new(Comm::from_raw_fd(read_fd, write_fd))?
            .main_loop()
            .map(|_| log::debug!("mockusbdev exit"))?;
        Ok(())
    }
}
