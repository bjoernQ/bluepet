use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;

pub fn handle_keyboard<O0, O1, O2, O3, O4, O5, O6, O7, I0, I1, I2, I3, I4, I5, I6, I7, I8, SHIFT>(
    out0: &mut O0,
    out1: &mut O1,
    out2: &mut O2,
    out3: &mut O3,
    out4: &mut O4,
    out5: &mut O5,
    out6: &mut O6,
    out7: &mut O7,
    in0: &I0,
    in1: &I1,
    in2: &I2,
    in3: &I3,
    in4: &I4,
    in5: &I5,
    in6: &I6,
    in7: &I7,
    in8: &I8,

    shift: &SHIFT,

    keyboard: &mut pet::io::Keyboard,
) where
    O0: OutputPin,
    O1: OutputPin,
    O2: OutputPin,
    O3: OutputPin,
    O4: OutputPin,
    O5: OutputPin,
    O6: OutputPin,
    O7: OutputPin,
    I0: InputPin,
    I1: InputPin,
    I2: InputPin,
    I3: InputPin,
    I4: InputPin,
    I5: InputPin,
    I6: InputPin,
    I7: InputPin,
    I8: InputPin,

    SHIFT: InputPin,
{
    let keycode = check_keyboard(
        out0, out1, out2, out3, out4, out5, out6, out7, in0, in1, in2, in3, in4, in5, in6, in7, in8,
    );

    let row_col = convert_keycode(keycode);

    if row_col == 0xff {
        for i in 0..0x9f {
            keyboard.key_up(i);
        }
    }

    if row_col != 0xff {
        keyboard.key_down(row_col);
    }

    // 85 = shift left
    if shift.is_high().unwrap_or_default() {
        keyboard.key_down(0x85);
    } else {
        keyboard.key_up(0x85);
    }
}

fn convert_keycode(hw_code: u8) -> u8 {
    match hw_code {
        1 => 0x00,
        2 => 0x10,
        3 => 0x01,
        4 => 0x11,
        5 => 0x02,
        6 => 0x12,
        7 => 0x03,
        8 => 0x13,
        9 => 0x04,
        10 => 0x14,
        11 => 0x05,
        12 => 0x06,
        13 => 0x07,
        14 => 0x16,
        15 => 0x36,
        16 => 0x17,
        17 => 0x37,
        18 => 0x27,
        19 => 0x26,
        20 => 0x25,
        21 => 0x34,
        22 => 0x24,
        23 => 0x33,
        24 => 0x31,
        25 => 0x23,
        26 => 0x32,
        27 => 0x22,
        28 => 0x21,
        29 => 0x30,
        30 => 0x20,
        31 => 0x57,
        32 => 0x47,
        33 => 0x44,
        34 => 0x56,
        35 => 0x46,
        36 => 0x54,
        37 => 0x53,
        38 => 0x43,
        39 => 0x52,
        40 => 0x42,
        41 => 0x51,
        42 => 0x77,
        43 => 0x41,
        44 => 0x50,
        45 => 0x40,
        46 => 0x67,
        47 => 0x76,
        48 => 0x66,
        49 => 0x65,
        50 => 0x74,
        51 => 0x63,
        52 => 0x64,
        53 => 0x73,
        54 => 0x72,
        55 => 0x62,
        56 => 0x71,
        57 => 0x61,
        58 => 0x70,
        59 => 0x97,
        60 => 0x86,
        61 => 0x87,
        62 => 0x96,
        63 => 0x60,
        64 => 0x94,
        65 => 0x84,
        66 => 0x93,
        67 => 0x92,
        68 => 0x91,
        69 => 0x84,
        70 => 0x81,
        71 => 0x90,
        72 => 0x82,
        _ => 0xff,
    }
}

fn check_keyboard<O0, O1, O2, O3, O4, O5, O6, O7, I0, I1, I2, I3, I4, I5, I6, I7, I8>(
    out0: &mut O0,
    out1: &mut O1,
    out2: &mut O2,
    out3: &mut O3,
    out4: &mut O4,
    out5: &mut O5,
    out6: &mut O6,
    out7: &mut O7,
    in0: &I0,
    in1: &I1,
    in2: &I2,
    in3: &I3,
    in4: &I4,
    in5: &I5,
    in6: &I6,
    in7: &I7,
    in8: &I8,
) -> u8
where
    O0: OutputPin,
    O1: OutputPin,
    O2: OutputPin,
    O3: OutputPin,
    O4: OutputPin,
    O5: OutputPin,
    O6: OutputPin,
    O7: OutputPin,
    I0: InputPin,
    I1: InputPin,
    I2: InputPin,
    I3: InputPin,
    I4: InputPin,
    I5: InputPin,
    I6: InputPin,
    I7: InputPin,
    I8: InputPin,
{
    let mut res = 0;

    out0.set_high().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 1;
    }
    if in1.is_high().unwrap_or_default() {
        res = 2;
    }
    if in2.is_high().unwrap_or_default() {
        res = 3;
    }
    if in3.is_high().unwrap_or_default() {
        res = 4;
    }
    if in4.is_high().unwrap_or_default() {
        res = 5;
    }
    if in5.is_high().unwrap_or_default() {
        res = 6;
    }
    if in6.is_high().unwrap_or_default() {
        res = 7;
    }
    if in7.is_high().unwrap_or_default() {
        res = 8;
    }
    if in8.is_high().unwrap_or_default() {
        res = 9;
    }
    out0.set_low().unwrap_or_default();

    out1.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 10;
    }
    if in1.is_high().unwrap_or_default() {
        res = 11;
    }
    if in2.is_high().unwrap_or_default() {
        res = 12;
    }
    if in3.is_high().unwrap_or_default() {
        res = 13;
    }
    if in4.is_high().unwrap_or_default() {
        res = 14;
    }
    if in5.is_high().unwrap_or_default() {
        res = 15;
    }
    if in6.is_high().unwrap_or_default() {
        res = 16;
    }
    if in7.is_high().unwrap_or_default() {
        res = 17;
    }
    if in8.is_high().unwrap_or_default() {
        res = 18;
    }
    out1.set_low().unwrap_or_default();

    out2.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 19;
    }
    if in1.is_high().unwrap_or_default() {
        res = 20;
    }
    if in2.is_high().unwrap_or_default() {
        res = 21;
    }
    if in3.is_high().unwrap_or_default() {
        res = 22;
    }
    if in4.is_high().unwrap_or_default() {
        res = 23;
    }
    if in5.is_high().unwrap_or_default() {
        res = 24;
    }
    if in6.is_high().unwrap_or_default() {
        res = 25;
    }
    if in7.is_high().unwrap_or_default() {
        res = 26;
    }
    if in8.is_high().unwrap_or_default() {
        res = 27;
    }
    out2.set_low().unwrap_or_default();

    out3.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 28;
    }
    if in1.is_high().unwrap_or_default() {
        res = 29;
    }
    if in2.is_high().unwrap_or_default() {
        res = 30;
    }
    if in3.is_high().unwrap_or_default() {
        res = 31;
    }
    if in4.is_high().unwrap_or_default() {
        res = 32;
    }
    if in5.is_high().unwrap_or_default() {
        res = 33;
    }
    if in6.is_high().unwrap_or_default() {
        res = 34;
    }
    if in7.is_high().unwrap_or_default() {
        res = 35;
    }
    if in8.is_high().unwrap_or_default() {
        res = 36;
    }
    out3.set_low().unwrap_or_default();

    out4.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 37;
    }
    if in1.is_high().unwrap_or_default() {
        res = 38;
    }
    if in2.is_high().unwrap_or_default() {
        res = 39;
    }
    if in3.is_high().unwrap_or_default() {
        res = 40;
    }
    if in4.is_high().unwrap_or_default() {
        res = 41;
    }
    if in5.is_high().unwrap_or_default() {
        res = 42;
    }
    if in6.is_high().unwrap_or_default() {
        res = 43;
    }
    if in7.is_high().unwrap_or_default() {
        res = 44;
    }
    if in8.is_high().unwrap_or_default() {
        res = 45;
    }
    out4.set_low().unwrap_or_default();

    out5.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 46;
    }
    if in1.is_high().unwrap_or_default() {
        res = 47;
    }
    if in2.is_high().unwrap_or_default() {
        res = 48;
    }
    if in3.is_high().unwrap_or_default() {
        res = 49;
    }
    if in4.is_high().unwrap_or_default() {
        res = 50;
    }
    if in5.is_high().unwrap_or_default() {
        res = 51;
    }
    if in6.is_high().unwrap_or_default() {
        res = 52;
    }
    if in7.is_high().unwrap_or_default() {
        res = 53;
    }
    if in8.is_high().unwrap_or_default() {
        res = 54;
    }
    out5.set_low().unwrap_or_default();

    out6.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    out7.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 55;
    }
    if in1.is_high().unwrap_or_default() {
        res = 56;
    }
    if in2.is_high().unwrap_or_default() {
        res = 57;
    }
    if in3.is_high().unwrap_or_default() {
        res = 58;
    }
    if in4.is_high().unwrap_or_default() {
        res = 59;
    }
    if in5.is_high().unwrap_or_default() {
        res = 60;
    }
    if in6.is_high().unwrap_or_default() {
        res = 61;
    }
    if in7.is_high().unwrap_or_default() {
        res = 62;
    }
    if in8.is_high().unwrap_or_default() {
        res = 63;
    }
    out6.set_low().unwrap_or_default();

    out7.set_high().unwrap_or_default();
    out0.set_low().unwrap_or_default();
    out2.set_low().unwrap_or_default();
    out3.set_low().unwrap_or_default();
    out4.set_low().unwrap_or_default();
    out5.set_low().unwrap_or_default();
    out6.set_low().unwrap_or_default();
    out1.set_low().unwrap_or_default();
    if in0.is_high().unwrap_or_default() {
        res = 64;
    }
    if in1.is_high().unwrap_or_default() {
        res = 65;
    }
    if in2.is_high().unwrap_or_default() {
        res = 66;
    }
    if in3.is_high().unwrap_or_default() {
        res = 67;
    }
    if in4.is_high().unwrap_or_default() {
        res = 68;
    }
    if in5.is_high().unwrap_or_default() {
        res = 69;
    }
    if in6.is_high().unwrap_or_default() {
        res = 70;
    }
    if in7.is_high().unwrap_or_default() {
        res = 71;
    }
    if in8.is_high().unwrap_or_default() {
        res = 72;
    }
    out7.set_low().unwrap_or_default();

    res
}
