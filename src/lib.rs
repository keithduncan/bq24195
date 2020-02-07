#![no_std]

use core::{
	fmt::Debug,
	default::Default,
	mem,
};

use embedded_hal::blocking::i2c::{
	Write,
	WriteRead,
};

use bitfield::bitfield;

pub struct Bq24195<I2C> {
	i2c: I2C,
}

pub enum Error<E> {
	I2C(E),
}

impl<E> Debug for Error<E> where E: Debug {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
		match self {
			Error::I2C(e) => write!(f, "Error::I2C({:?})", e),
		}
	}
}

const ADDRESS: u8 = 0x6B;

#[allow(unused)]
enum Register {
	InputSourceControl = 0x00, // r/w
	PowerOnConfiguration = 0x01, // r/w
	ChargeCurrentControl = 0x02, // r/w
	PreChargeTerminationCurrentControl = 0x03, // r/w
	ChargeVoltageControl = 0x04, // r/w
	ChargeTerminationTimerControl = 0x05, // r/w
	ThermalRegulationControl = 0x06, // r/w
	MiscOperationControl = 0x07, //r/w
	SystemStatus = 0x08, // ro
	Fault = 0x09, // ro
	VendorPartRevisionStatus = 0x0A, // ro
}

#[derive(Debug)]
#[allow(unused)]
#[repr(u8)]
pub enum InputVoltageLimit {
	V3_88 = 0b0000,
	V3_96 = 0b0001,
	V4_04 = 0b0010,
	V4_12 = 0b0011,
	V4_2  = 0b0100,
	V4_28 = 0b0101,
	V4_36 = 0b0110,
	V4_44 = 0b0111,
	V4_52 = 0b1000,
	V4_6  = 0b1001,
	V4_68 = 0b1010,
	V4_76 = 0b1011,
	V4_84 = 0b1100,
	V4_92 = 0b1101,
	V5    = 0b1110,
	V5_08 = 0b1111,
}

impl Into<u8> for InputVoltageLimit {
	fn into(self) -> u8 {
		self as u8
	}
}

impl From<u8> for InputVoltageLimit {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b111) }
	}
}

#[derive(Debug)]
#[allow(unused)]
#[repr(u8)]
pub enum InputCurrentLimit {
	MA100  = 0b000,
	MA150  = 0b001,
	MA500  = 0b010,
	MA900  = 0b011,
	MA1200 = 0b100,
	MA1500 = 0b101,
	MA2000 = 0b110,
	MA3000 = 0b111,
}

impl Into<u8> for InputCurrentLimit {
	fn into(self) -> u8 {
		self as u8
	}
}

impl From<u8> for InputCurrentLimit {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b111) }
	}
}

bitfield! {
	pub struct InputSourceControl(u8);
	impl Debug;

	pub bool, hiz,    set_hiz    : 7;
	pub u8, from into InputVoltageLimit, input_voltage_limit, set_input_voltage_limit : 6, 3;
	pub u8, from into InputCurrentLimit, input_current_limit, set_input_current_limit : 2, 0;
}

impl Default for InputSourceControl {
	fn default() -> Self {
		let mut reg = InputSourceControl(0);
		reg.set_hiz(false);
		reg.set_input_voltage_limit(InputVoltageLimit::V4_36);
		reg.set_input_current_limit(InputCurrentLimit::MA100);
		reg
	}
}

#[derive(Debug)]
#[allow(unused)]
#[repr(u8)]
pub enum ChargerConfiguration {
	ChargeDisabled = 0b00,
	ChargeBattery = 0b01,
	OTG = 0b10, // can also be 0b11
}

impl Into<u8> for ChargerConfiguration {
	fn into(self) -> u8 {
		self as u8
	}
}

impl From<u8> for ChargerConfiguration {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b11) }
	}
}

#[derive(Debug)]
#[allow(unused)]
#[repr(u8)]
pub enum MinimumSystemVoltage {
	V3   = 0b000,
	V3_1 = 0b001,
	V3_2 = 0b010,
	V3_3 = 0b011,
	V3_4 = 0b100,
	V3_5 = 0b101,
	V3_6 = 0b110,
	V3_7 = 0b111,
}

impl Into<u8> for MinimumSystemVoltage {
	fn into(self) -> u8 {
		self as u8
	}
}

impl From<u8> for MinimumSystemVoltage {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b111) }
	}
}

bitfield! {
	pub struct PowerOnConfiguration(u8);
	impl Debug;

	pub bool, register_reset, set_reset : 7;
	pub bool, watchdog_reset, set_watchdog_reset : 6;
	pub u8, from into ChargerConfiguration, charger_configuration, set_charger_configuration : 5, 4;
	pub u8, from into MinimumSystemVoltage, minimum_system_voltage, set_minimum_system_voltage : 3, 1;
}

impl Default for PowerOnConfiguration {
	fn default() -> Self {
		// Always set lsb to 1
		let mut reg = PowerOnConfiguration(0b0000_0001);
		reg.set_charger_configuration(ChargerConfiguration::ChargeBattery);
		reg.set_minimum_system_voltage(MinimumSystemVoltage::V3_5);
		reg
	}
}

bitfield! {
	pub struct MiscOperationControl(u8);
	impl Debug;

	pub bool, dpdm_detection, set_dpdm_detection : 7;
	pub bool, safety_timer_slowed, set_safety_timer_slowed : 6;
	pub bool, battery_fet_disabled, set_battery_fet_disabled : 5;
	pub bool, charge_fault_interrupt, set_charge_fault_interrupt : 1;
	pub bool, battery_fault_interrupt, set_battery_fault_interrupt : 0;
}

impl Default for MiscOperationControl {
	fn default() -> Self {
		let mut reg = MiscOperationControl(0b00001000);
		reg.set_charge_fault_interrupt(true);
		reg.set_battery_fault_interrupt(true);
		reg
	}
}

impl<I2C, E> Bq24195<I2C>
	where I2C: Write<Error = E> {
	/// Create a new driver instance.
	///
	/// i2c: An i2c bus connected to the Bq24195 chip. Bq24195 supports both
	/// 400khz and 100khz operation.
	pub fn new(i2c: I2C) -> Self {
		Self {
			i2c,
		}
	}

	pub fn set_input_source_control(&mut self, input_source_control: InputSourceControl) -> Result<(), Error<E>> {
		self.write_register(Register::InputSourceControl, input_source_control.0)
	}

	pub fn set_power_on_configuration(&mut self, power_on_configuration: PowerOnConfiguration) -> Result<(), Error<E>> {
		self.write_register(Register::PowerOnConfiguration, power_on_configuration.0)
	}

	pub fn set_misc_operation_control(&mut self, misc_operation_control: MiscOperationControl) -> Result<(), Error<E>> {
		self.write_register(Register::MiscOperationControl, misc_operation_control.0)
	}

	fn write_register(&mut self, register: Register, value: u8) -> Result<(), Error<E>> {
        self.i2c
            .write(ADDRESS, &[register as u8, value])
            .map_err(Error::I2C)?;
        Ok(())
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum VbusStatus {
	Unknown = 0b00,
	UsbHost = 0b01,
	Adapter = 0b10,
	Otg     = 0b11,
}

impl From<u8> for VbusStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b111) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum ChargeStatus {
	NotCharging = 0b00,
	PreCharge   = 0b01,
	FastCharge  = 0b10,
	ChargeDone  = 0b11,
}

impl From<u8> for ChargeStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b11) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum DpmStatus {
	NotDynamicPowerManagement = 0b0,
	Vindpm = 0b1,
}

impl From<u8> for DpmStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum PowerStatus {
	NotGood = 0b0,
	Good    = 0b1,
}

impl From<u8> for PowerStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum ThermalStatus {
	Normal    = 0b0,
	Regulated = 0b1,
}

impl From<u8> for ThermalStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum VsysStatus {
	NotRegulated = 0b0,
	Regulated    = 0b1,
}

impl From<u8> for VsysStatus {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

bitfield! {
	pub struct SystemStatus(u8);
	impl Debug;

	pub u8, into VbusStatus, vbus_status, _ : 7, 6;
	pub u8, into ChargeStatus, charge_status, _ : 5, 4;
	pub u8, into DpmStatus, dpm_status, _ : 3, 3;
	pub u8, into PowerStatus, power_status, _ : 2, 2;
	pub u8, into ThermalStatus, thermal_status, _ : 1, 1;
	pub u8, into VsysStatus, vsys_status, _ : 0, 0;
}

#[derive(Debug)]
#[repr(u8)]
pub enum Watchdog {
	Normal  = 0b0,
	Expired = 0b1,
}

impl From<u8> for Watchdog {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum ChargeFault {
	Normal             = 0b00,
	InputFault         = 0b01,
	ThermalShutdown    = 0b10,
	SafetyTimerExpired = 0b11,
}

impl From<u8> for ChargeFault {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b11) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum BatteryFault {
	Normal               = 0b0,
	OverVoltageThreshold = 0b1,
}

impl From<u8> for BatteryFault {
	fn from(val: u8) -> Self {
		unsafe { mem::transmute(val & 0b1) }
	}
}

#[derive(Debug)]
#[repr(u8)]
pub enum ThermistorFault {
	Normal,
	Cold,
	Hot,
	Unknown,
}

impl From<u8> for ThermistorFault {
	fn from(val: u8) -> Self {
		match val & 0b111 {
			0b000 => ThermistorFault::Normal,
			0b101 => ThermistorFault::Cold,
			0b110 => ThermistorFault::Hot,
			_     => ThermistorFault::Unknown,
		}
	}
}

bitfield! {
	pub struct Fault(u8);
	impl Debug;

	pub u8, into Watchdog, watchdog, _ : 7, 7;
	pub u8, into ChargeFault, charge_fault, _ : 5, 4;
	pub u8, into BatteryFault, battery_fault, _ : 3, 3;
	pub u8, into ThermistorFault, thermistor_fault, _ : 2, 0;
}

impl<I2C, E> Bq24195<I2C>
	where I2C: WriteRead<Error = E> {
	pub fn system_status(&mut self) -> Result<SystemStatus, Error<E>> {
		let val = self.read_register(Register::SystemStatus)?;
		Ok(SystemStatus(val))
	}

	pub fn fault(&mut self) -> Result<Fault, Error<E>> {
		let val = self.read_register(Register::Fault)?;
		Ok(Fault(val))
	}

	fn read_register(&mut self, register: Register) -> Result<u8, Error<E>> {
        let mut data = [0; 1];
        self.i2c
            .write_read(ADDRESS, &[register as u8], &mut data)
            .map_err(Error::I2C)?;
        Ok(data[0])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
