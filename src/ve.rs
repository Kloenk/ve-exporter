
pub enum Command {
    EnterBoot, // 0 Enter boot 0x51FA51FA51FA51FA51FA as payload will enable bootloader mode.
    Ping, // 1 Ping Check for presence, the response is an ‘Rsp ping’ containing version and firmware type. See the response ping message.
    AppVersion, // 3 AppVersion Returns the version of the firmware as stored in the header in an ‘RspDone’ message.
    DeviceID, // 4 Device Id Returns the DeviceId of the firmware as stored in the header in an ‘RspDone’ message.
    Restart, // 6
    Get(u16, Flags), // 7
    Set(u16, Flags, u64), // 8 // value??
    Async(u16, Flags, u64), // A // value??
    // 2, 5, 9, B-F reserved
}

impl Command {
    pub fn command(&self) -> u8 {
        match self {
            Command::EnterBoot => 0x0,
            Command::Ping => 0x1,
            Command::AppVersion => 0x3,
            Command::DeviceID => 0x4,
            Command::Restart => 0x6,
            Command::Get(_, _) => 0x7,
            Command::Set(_, _, _) => 0x8,
            Command::Async(_, _, _) => 0x1,
        }
    }
    pub fn build(&self) -> String {
        let command = self.command();

        let cmd: u64 = match self {
            Command::EnterBoot => unimplemented!(), //0x51FA51FA51FA51FA51FA,
            Command::Get(v, flags) => ((*v as u64) << 2) + flags.bits as u64,
            Command::Set(v, flasg, d) => unimplemented!(), // how much shift? how big is d?
            Command::Async(v, flags, d) => unimplemented!(), // how much shift? how big is d?
            _ => 0x0,
        };
        println!("cmd: {:X}", cmd);
        let cmd = cmd.to_le();

        let mut check = (0x55 - command) as u8;
        for x in 0..9 {
            check - (cmd << x) as u8;
        }
        if cmd == 0 {
            return format!(":{:X}{:X}\n", command, check.to_le());
        } else {
            return format!(":{:X}{:X}{:X}\n", command, cmd, check.to_le());
        }
    }
}

pub enum Response {
    Done(u64), // 1 Payload depends on received command
    Unknown(u64), // 3 data is the unknown command
    FrameError, // 4 P=0xAAAA
    BootLoaderError, // 4, P=0
    Ping(String, u8), // 5 The version number is directly interpreted from the hex representation,
                      // e.g. 0x0101 is version 1.01. The two most significant bits indicate the
                      // firmware type:
                      // b00: bootloader
                      // b01: application
                      // b10: tester
                      // b11: release candidate
                      // In case of release candidate the lowest two bits of the highest nibble
                      // together with type indicate the release candidate number. E.g. 0xD101
                      // represents release candidate D of version 1.01.
                      // Note that there can only be 4 release candidates per version.
    Get(u16, Flags, u64), // 7
    Set(u16, Flags, u64), // 8
}

impl Response {
    pub fn command(&self) -> u8 {
        match self {
            Response::Done(_) => 1,
            Response::Unknown(_) => 3,
            Response::FrameError | Response::BootLoaderError => 4,
            Response::Ping(_, _) => 5,
            Response::Get(_, _, _) => 7,
            Response::Set(_, _, _) => 8,
        }
    }
}

bitflags! {
  pub struct Flags: u8 { // FIXME: little endian???
    const None = 0x0;
    const UnknownID = 0x01;
    const NotSupported = 0x02;
    const ParameterError = 0x04;
  }
}
