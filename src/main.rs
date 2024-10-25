//! Blinks the LED on a Pico board and plays a midi note on/off message.
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico::{self as bsp, hal::Timer};
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{clocks::init_clocks_and_plls, pac, sio::Sio, usb::UsbBus, watchdog::Watchdog};

// Import necessary types from the usb-device crate
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::*;
// use usbd_midi::data::usb_midi::constants::*;
use usbd_midi::{
    data::{
        byte::u7::U7,
        midi::{message::Message, notes::Note},
        usb_midi::{cable_number::CableNumber, usb_midi_event_packet::UsbMidiEventPacket},
    },
    midi_device::MidiClass,
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    // let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.
    let mut led_pin = pins.led.into_push_pull_output();

    // Must be of type `usb_device::bus::UsbBusAllocator`.
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Create a MIDI class with 1 input and 1 output jack.
    let mut midi = MidiClass::new(&usb_bus, 1, 1).expect("Failed to create MIDI class");

    let string_descriptors = [StringDescriptors::default()
        .manufacturer("Music Company")
        .product("MIDI Device")
        .serial_number("12345678")];

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x5e4))
        .strings(&string_descriptors)
        .expect("Failed to build USB device from descriptions")
        .device_class(0)
        .device_sub_class(0)
        .build();

    let mut next_toggle = timer.get_counter().ticks() + 500_000; // 500 ms in microseconds
    let mut led_on = false;
    loop {
        // Poll the USB device and MIDI class
        if !usb_dev.poll(&mut [&mut midi]) {
            // Handle MIDI events here
            continue;
        }

        let now = timer.get_counter().ticks();
        if now >= next_toggle {
            next_toggle += 500_000; // Schedule next toggle in 500 ms

            if led_on {
                info!("off!");
                led_pin.set_low().unwrap();

                // Send MIDI Note Off message for note 48 (C3)
                let cable_number = CableNumber::Cable0;
                let channel = usbd_midi::data::midi::channel::Channel::Channel1;
                let note = Note::C3;
                let velocity = U7::MAX;
                let note_off = Message::NoteOff(channel, note, velocity);
                let note_off_message = UsbMidiEventPacket::from_midi(cable_number, note_off);
                midi.send_message(note_off_message).unwrap();
            } else {
                info!("on!");
                led_pin.set_high().unwrap();

                // Send MIDI Note On message for note 48 (C3)
                let cable_number = CableNumber::Cable0;
                let channel = usbd_midi::data::midi::channel::Channel::Channel1;
                let note = Note::C3;
                let velocity = U7::MAX;
                let note_on = Message::NoteOn(channel, note, velocity);
                let note_on_message = UsbMidiEventPacket::from_midi(cable_number, note_on);
                midi.send_message(note_on_message).unwrap();
            }

            led_on = !led_on;
        }
    }
}
