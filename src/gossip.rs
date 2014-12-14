#![feature(phase)]
#[phase(plugin, link)]
extern crate log;

#[deriving(Copy, Eq, PartialEq, Clone, Show)]
enum State {
    Idle,
    SPIEnable, 
    SPITransfer,
    I2CEnable,
    I2CWrite,
    I2CRead,
    UARTEnable,
    UARTTransfer,
    UARTReceive,
    ExpectRepeatCommand,
}

mod commands {
    // Base Addr
    pub const CMD_BASE: u8 =                0x80;

    // General Ops
    pub const CMD_NOP: u8 =                 0x00;
    pub const CMD_SLEEP: u8 =               0x10 | CMD_BASE;

    // SPI
    pub const CMD_SPIENABLE: u8 =           0x20 | CMD_BASE;
    pub const CMD_SPITRANSFER: u8 =         0x21 | CMD_BASE;
    pub const CMD_SPIDISABLE: u8 =          0x22 | CMD_BASE;
    pub const CMD_SPISETCLOCKDIVISER: u8 =  0x23 | CMD_BASE;
    pub const CMD_SPISETSPIMODE: u8 =       0x24 | CMD_BASE;
    pub const CMD_SPISETROLE: u8 =          0x25 | CMD_BASE;
    pub const CMD_SPISETFRAME: u8 =         0x26 | CMD_BASE;

    // I2C
    pub const CMD_I2CENABLE: u8 =           0x30 | CMD_BASE;
    pub const CMD_I2CWRITE: u8 =            0x31 | CMD_BASE;
    pub const CMD_I2CREAD: u8 =             0x32 | CMD_BASE;
    pub const CMD_I2CDISABLE: u8 =          0x33 | CMD_BASE;
    pub const CMD_SPISETMODE: u8 =          0x34 | CMD_BASE;
    pub const CMD_SPISETSLAVEADDRESS: u8 =  0x35 | CMD_BASE;

    // UART
    pub const CMD_UARTENABLE: u8 =          0x40 | CMD_BASE;
    pub const CMD_UARTTRANSFER: u8 =        0x41 | CMD_BASE;
    pub const CMD_UARTRECEIVE: u8 =         0x42 | CMD_BASE;
    pub const CMD_UARTDISABLE: u8 =         0x43 | CMD_BASE;
    pub const CMD_UARTSETBAUDRATE: u8 =     0x44 | CMD_BASE;
    pub const CMD_UARTSETDATABITS: u8 =     0x45 | CMD_BASE;
    pub const CMD_UARTSETPARITY: u8 =       0x46 | CMD_BASE;
    pub const CMD_UARTSETSTOPBITS: u8 =     0x47 | CMD_BASE;

}

trait SPI {
  fn enable();
  fn transfer(byte: u8) -> u8;
  fn disable();
  fn setClockSpeedDivisor(divisor: u8);
  fn setMode(mode: u8);
  fn setRole(role: u8);
  fn setFrame(frame: u8);
}


struct IOStateMachine {
    state : State,
    repeat_remaining : u8,
}

impl IOStateMachine {

    fn is_repeat_token(&mut self, byte: u8) -> bool {
        byte < commands::CMD_BASE
    }

    fn is_valid_repeat_state(&mut self) -> bool {
        (self.state == State::SPIEnable 
        || self.state == State::I2CEnable
        || self.state == State::UARTEnable)
    }

    fn handle_byte(&mut self, byte: u8) {
        debug!("Received byte {}", byte);

        // If this is a repeat command
        if byte != 0 && self.is_repeat_token(byte) && self.is_valid_repeat_state() {
            // Set the number of times to repeat
            self.repeat_remaining = byte;
            // Set the state to be expecting the command to repeat
            self.state = State::ExpectRepeatCommand;
            return
        }
        // Repeat number has been set and we need to set the command that we will be repeating
        else if self.repeat_remaining != 0 && self.state == State::ExpectRepeatCommand {
            match byte {
                // If it's a nop, do it now...
                commands::CMD_NOP => { 
                    while self.repeat_remaining > 0 {
                        nop();
                        self.repeat_remaining-=1;
                    };
                    self.state = State::Idle;
                },
                // If it's a sleep command, do it now...
                commands::CMD_SLEEP => { 
                    while self.repeat_remaining > 0 {
                        sleep();
                        self.repeat_remaining-=1;
                    };
                    self.state = State::Idle;
                },
                commands::CMD_SPITRANSFER => {
                    self.state = State::SPITransfer;
                },
                commands::CMD_I2CWRITE => {
                    self.state = State::I2CWrite;
                },
                commands::CMD_I2CREAD => {
                    self.state = State::I2CRead;
                },
                commands::CMD_UARTTRANSFER => {
                    self.state = State::UARTTransfer;
                },
                _ => nop(),
            } 

            return;
        }

        // This is a standard, one-time only command
        else if self.repeat_remaining == 0 {
            self.repeat_remaining = 1;
        }

        // Decrement the count
        self.repeat_remaining-=1;

        // Find the appropriate state to transfer to 
        match (self.state, byte) {
            (State::Idle, commands::CMD_NOP) => nop(),
            (State::Idle, commands::CMD_SLEEP) => sleep(),
            (State::Idle, commands::CMD_SPIENABLE) => { 
                // spi.enable();
                self.state = State::SPIEnable;
            },
            (State::SPIEnable, commands::CMD_SPITRANSFER) => {
                self.state = State::SPITransfer;
            },
            (State::SPITransfer, _) => {
                // spi.transfer(byte);

                if (self.repeat_remaining == 0) {
                    self.state = State::SPIEnable;
                }
            },
            (State::SPIEnable, commands::CMD_SPIDISABLE) => {
                // spi.disable();
                self.state = State::Idle;
            },
            (State::Idle, commands::CMD_I2CENABLE) => {
                // i2c.enable();
                self.state = State::I2CEnable;
            },
            (State::I2CEnable, commands::CMD_I2CWRITE) => {
                self.state = State::I2CWrite;
            },
            (State::I2CEnable, commands::CMD_I2CREAD) => {
                // let ret = i2c.read();
                if (self.repeat_remaining == 0) {
                    self.state = State::I2CEnable;
                }
            },
            (State::I2CWrite, _) => {
                // i2c.write(byte);
                if (self.repeat_remaining == 0) {
                    self.state = State::I2CEnable;
                }
            },
            (State::I2CRead, _) => {
                // let ret = i2c.read(byte);
                if (self.repeat_remaining == 0) {
                    self.state = State::I2CEnable;
                }
            },
            (State::I2CEnable, commands::CMD_I2CDISABLE) => {
                // i2c.disable();
                self.state = State::Idle;
            },
            (State::Idle, commands::CMD_UARTENABLE) => {
                // uart.enable();
                self.state = State::UARTEnable;
            },
            (State::UARTEnable, commands::CMD_UARTTRANSFER) => {
                self.state = State::UARTTransfer;
            },
            (State::UARTTransfer, _) => {
                // uart.transfer(byte);
                if (self.repeat_remaining == 0) {
                    self.state = State::UARTEnable;
                }
            },
            (State::UARTEnable, commands::CMD_UARTDISABLE) => {
                // uart.disable();
                self.state = State::Idle;
            },
            _ => nop(),
        }

    }

    // fn return_byte(&self, ) {

    // }
}

fn nop() {
}

fn sleep() {
    nop();
}

struct Mock_SPI {
    enable: bool;
    clock_speed_divisor: u8;
    out_reg: u8;
    mode: u8;
    frame: u8;
    role: u8;
}

impl SPI for Mock_SPI {
  fn transfer(&mut self, byte: u8) -> u8 {
    if self.enable {
        self.out_reg = byte;
    }
  }
  fn enable() {
    self.enable = true;
  }
  fn disable() {
    self.enable = false;
  }
  fn setClockSpeedDivisor(divisor: u8) {
    self.clock_speed_divisor = divisor;
  }
  fn setMode(mode: u8) {
    self.mode = mode;
  }
  fn setRole(role: u8) {
    self.role = role;
  }
  fn setFrame(frame: u8) {
    self.frame = frame;
  }
}


//#[cfg(test)]
mod test {
    use super::State;
    use super::IOStateMachine;
    use super::commands;


    #[test]
    fn test_handle_idle_spi_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_repeat_token() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        assert_eq!(s.is_repeat_token(254), false);
        assert_eq!(s.is_repeat_token(0), true);
    }

    #[test]
    fn test_repeat_nop() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(100);
        s.handle_byte(commands::CMD_NOP);
        assert_eq!(s.state, State::Idle);
    }

    #[test]
    fn test_repeat_sleep() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(100);
        s.handle_byte(commands::CMD_SLEEP);
        assert_eq!(s.state, State::Idle);
    }

    #[test]
    fn test_handle_spi_enable_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_handle_spi_transfer() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::SPITransfer);
        s.handle_byte(200);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_handle_spi_transfer_repeat() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(2);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::SPITransfer);
        s.handle_byte(200);
        assert_eq!(s.state, State::SPITransfer);
        s.handle_byte(200);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(200);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_handle_spi_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(commands::CMD_SPIDISABLE);
        assert_eq!(s.state, State::Idle);
    }

    #[test]
    fn test_handle_spi_transfer_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::SPITransfer);
        s.handle_byte(commands::CMD_SPIDISABLE);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_handle_spi_transfer_repeat_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(2);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::SPITransfer);
        s.handle_byte(commands::CMD_SPIDISABLE);
        assert_eq!(s.state, State::SPITransfer);
    }

    #[test]
    fn test_handle_i2c_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_write() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(commands::CMD_I2CWRITE);
        assert_eq!(s.state, State::I2CWrite);
        s.handle_byte(100);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_write_repeat() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        let repeat: u8 = 5;
        s.handle_byte(repeat);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_I2CWRITE);
        assert_eq!(s.state, State::I2CWrite);
        for i in range(0, repeat-1) {
            debug!("Sending again {}", i);
            s.handle_byte(i);
            assert_eq!(s.state, State::I2CWrite);
        }
        s.handle_byte(200);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_read() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(commands::CMD_I2CREAD);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_read_repeat() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        let repeat: u8 = 5;
        s.handle_byte(repeat);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_I2CREAD);
        assert_eq!(s.state, State::I2CRead);
        for i in range(0, repeat-1) {
            s.handle_byte(0);
            assert_eq!(s.state, State::I2CRead);
        }
        s.handle_byte(200);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(commands::CMD_I2CDISABLE);
        assert_eq!(s.state, State::Idle);
    }

     #[test]
    fn test_handle_i2c_write_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(commands::CMD_I2CWRITE);
        assert_eq!(s.state, State::I2CWrite);
        s.handle_byte(commands::CMD_I2CDISABLE);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_handle_i2c_write_repeat_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(2);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_I2CWRITE);
        assert_eq!(s.state, State::I2CWrite);
        s.handle_byte(commands::CMD_SPIDISABLE);
        assert_eq!(s.state, State::I2CWrite);
    }

    #[test]
    fn test_handle_uart_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
    }

    #[test]
    fn test_handle_uart_transfer() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
        s.handle_byte(commands::CMD_UARTTRANSFER);
        assert_eq!(s.state, State::UARTTransfer);
        s.handle_byte(200);
        assert_eq!(s.state, State::UARTEnable);
    }

    #[test]
    fn test_handle_uart_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
        s.handle_byte(commands::CMD_UARTDISABLE);
        assert_eq!(s.state, State::Idle);
    }

     #[test]
    fn test_handle_uart_write_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
        s.handle_byte(commands::CMD_UARTTRANSFER);
        assert_eq!(s.state, State::UARTTransfer);
        s.handle_byte(commands::CMD_UARTDISABLE);
        assert_eq!(s.state, State::UARTEnable);
    }

    #[test]
    fn test_handle_uart_write_repeat_disable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
        s.handle_byte(2);
        assert_eq!(s.state, State::ExpectRepeatCommand);
        s.handle_byte(commands::CMD_UARTTRANSFER);
        assert_eq!(s.state, State::UARTTransfer);
        s.handle_byte(commands::CMD_UARTDISABLE);
        assert_eq!(s.state, State::UARTTransfer);
    }

    #[test]
    fn test_handle_spi_write_while_uart_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_UARTENABLE);
        assert_eq!(s.state, State::UARTEnable);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::UARTEnable);
    }

    #[test]
    fn test_handle_spi_transfer_while_idle() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.state, State::Idle);
    }

    #[test]
    fn test_handle_spi_enable_while_i2c_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_I2CENABLE);
        assert_eq!(s.state, State::I2CEnable);
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::I2CEnable);
    }

    #[test]
    fn test_zero_repeat_in_spi_enable() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.state, State::SPIEnable);
        s.handle_byte(0);
        assert_eq!(s.state, State::SPIEnable);
    }

    #[test]
    fn test_valid_state() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        assert_eq!(s.is_valid_repeat_state(), false);
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.is_valid_repeat_state(), true);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.is_valid_repeat_state(), false);

    }

    #[test]
    fn test_spi_set_clock_divisor() {
        let mut s = IOStateMachine{state:State::Idle, repeat_remaining : 0};
        s.handle_byte(commands::CMD_SPI);
        assert_eq!(s.is_valid_repeat_state(), false);
        s.handle_byte(commands::CMD_SPIENABLE);
        assert_eq!(s.is_valid_repeat_state(), true);
        s.handle_byte(commands::CMD_SPITRANSFER);
        assert_eq!(s.is_valid_repeat_state(), false);

    }
}
