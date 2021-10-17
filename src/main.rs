use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::env;

fn main() {
    let mut args = env::args();
    let nameopt = args.next().unwrap();
    let destopt = args.next();
    let endianopt = args.next();
    if destopt.is_none() || endianopt.is_none() {
	usage(&nameopt);
	return;
    }

    let dest : u8;
    let endian : u8;
    match destopt.unwrap().as_str() {
	"-f" => dest = 8, // UTF-32 -> UTF-8
	"-t" => dest = 32,// UTF-8 -> UTF-32
	_ => { usage(&nameopt); return },
    }
    match endianopt.unwrap().as_str() {
	"-b" => endian = 1,   // Big
	"-l" => endian = 0,   // Little
	"-s" => endian = 255, // Swap
	_ => { usage(&nameopt); return },
    }

    if (dest==8) && (endian==255) { // Prevent endian swap when destination is UTF-8
	usage(&nameopt);
	return;
    }

    let mut reader = BufReader::new(io::stdin());
    let mut writer = BufWriter::new(io::stdout());

    let mut inp : Vec<u8>;
    let mut out : Vec<u8> = Vec::new();

    if dest == 8 { // UTF-32 to UTF-8
        let mut buf : [u8; 4] = [0,0,0,0];

        while reader.read_exact(&mut buf).is_ok() {
	    if endian == 0 { // If UTF-32LE, convert to UTF-32BE
	    	inp = Vec::new();
		inp.push(buf[3]);
		inp.push(buf[2]);
		inp.push(buf[1]);
		inp.push(buf[0]);
	    }
	    else {
       	        inp = buf.to_vec();
	    }
	    for _n in 1..4 {
	        if inp.get(0).unwrap() == &0 {
		    inp.remove(0);
	        }
	    }
    	    to8(inp, &mut out);
        }
    }
    else if endian != 255 { // UTF-8 to UTF-32
	let mut tmp : [u8;1] = [0];
	let mut val : u32;
	while reader.read_exact(&mut tmp).is_ok() {
		inp = Vec::new();
		inp.push(tmp[0]); // All that is needed for 00-7F
		if (tmp[0] & 0b10000000) >> 7 == 0 { // One Byte
		}
		else if (tmp[0] & 0b11000000) >> 6 == 0b10 {
		    panic!("Invalid UTF-8");
		}
		else if (tmp[0] & 0b11100000) >> 5 == 0b110 { // Two Byte
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		}
		else if (tmp[0] & 0b11110000) >> 4 == 0b1110 { // Three Byte
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		}
		else if (tmp[0] & 0b11111000) >> 3 == 0b11110 { // Four Byte
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		    reader.read_exact(&mut tmp).unwrap();
		    inp.push(tmp[0]);
		}
		else {
		    panic!("Invalid UTF-8");
		}

	    val = to32(&mut inp);
 	    if endian == 1 {
  	        out.push(((val & 0xFF000000) >> 24) as u8);
	        out.push(((val & 0xFF0000) >> 16) as u8);
	        out.push(((val & 0xFF00) >> 8) as u8);
	        out.push((val & 0xFF) as u8);
	    }
	    else {
	        out.push((val & 0xFF) as u8);
	        out.push(((val & 0xFF00) >> 8) as u8);
	        out.push(((val & 0xFF0000) >> 16) as u8);
	        out.push(((val & 0xFF000000) >> 24) as u8);
	    }
	}
    }
    else { // UTF-32 Endian Swap
	let mut tmp : [u8; 4] = [0,0,0,0];
	while reader.read_exact(&mut tmp).is_ok() {
	    out.push(tmp[3]);
	    out.push(tmp[2]);
	    out.push(tmp[1]);
	    out.push(tmp[0]);
	}
    }
    writer.write(&out).expect("Failed to write output.");
}

fn usage(name : &str) {
    eprintln!("UTF-32 <-> UTF-8 file converter\nUsage: {} [-f: UTF-32 -> UTF-8]||[-t: UTF-8 -> UTF-32 or UTF-32BE <-> UTF-32LE] [-b: UTF-32BE]||[-l: UTF-32LE]||[-s: UTF-32BE <-> UTF-32LE]  < input > output", name);
}

fn to32(cp : &mut Vec<u8>) -> u32 { // UTF-8 to UTF-32BE
    let mut tmp : u32;
    let mut out : u32 = 0;
    tmp = cp.remove(0) as u32;
    if (tmp & 0b10000000) >> 7 == 0 { // 00-7F
	out = tmp;
    }
    else if (tmp & 0b11000000) >> 6 == 0b10 {
    	panic!("Invalid UTF-8");
    }
    else if (tmp & 0b11100000) >> 5 == 0b110 { // 80-7FF
        // 110XXXXX 10XXXXXX
        // 00000000 00000000 00000XXX XXXXXXXX
        // 11 data bytes
        out |= (tmp & 0b00011111) << 6;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= tmp & 0b00111111;
    }
    else if (tmp & 0b11110000) >> 4 == 0b1110 { // 800-FFFF
        // 1110XXXX 10XXXXXX 10XXXXXX
        // 00000000 00000000 XXXXXXXX XXXXXXXX
        // 16 data bytes
        out |= (tmp & 0b00001111) << 12;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= (tmp & 0b00111111) << 6;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= tmp & 0b001111111;
    }
    else if (tmp & 0b11111000) >> 3 == 0b11110 {  // 10000-10FFFF
        // 11110XXX 10XXXXXX 10XXXXXX 10XXXXXX
        // 00000000 000XXXXX XXXXXXXX XXXXXXXX
        // 21 data bytes
        out |= (tmp & 0b00000111) << 18;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= (tmp & 0b00111111) << 12;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= (tmp & 0b00111111) << 6;
        tmp = cp.remove(0) as u32; // Fresh Byte
        out |= tmp & 0b00111111;
    }
    else {
    	panic!("Invalid UTF-8");
    }
    return out;
}

fn to8(mut cp : Vec<u8>, out : &mut Vec<u8>) { // UTF-32BE to UTF-8
    let mut tmp : u8;
    let mut wtmp : u8;
    match cp.len() {
	1 => {
	    tmp = cp.remove(0);
	    if tmp > 0x7F { // 1 : 2
		// XXXXXXXX
		// 110000XX 10XXXXXX
		wtmp = 0b11000000;
		wtmp |= (tmp & 0b11000000) >> 6;
		out.push(wtmp);

		wtmp = 0b10000000;
		wtmp |= tmp & 0b111111;
		out.push(wtmp);
	    }	
	    else { // 1 : 1
		// 0XXXXXXX
		// 0XXXXXXX
		out.push(tmp);
	    }
	}
	2 => {
	    tmp = cp.remove(0);
	    if tmp > 0b111 { // 2 : 3
		// XXXXXXXX XXXXXXXX
		// 1110XXXX 10XXXXXX 10XXXXXX
		wtmp = 0b11100000;
		wtmp |= tmp >> 4;
		out.push(wtmp);
		
		wtmp = 0b10000000;
		wtmp |= (tmp & 0b1111) << 2;
		tmp = cp.remove(0); // Fresh Byte
		wtmp |= (tmp & 0b11000000) >> 6;
		out.push(wtmp);

		wtmp = 0b10000000;
		wtmp |= tmp & 0b111111;
		out.push(wtmp);
	    }
	    else { // 2 : 2
		// 00000XXX XXXXXXXX
		// 110XXXXX 10XXXXXX
		wtmp = 0b11000000;
		wtmp |= tmp << 2;
		tmp = cp.remove(0); // Fresh Byte
		wtmp |= tmp >> 6;
		out.push(wtmp);
		
		wtmp = 0b10000000;
		wtmp |= tmp & 0b111111;
		out.push(wtmp);
	    }
	}
	3 => { // 3 : 4
	    tmp = cp.remove(0);
	    // 000XXXXX XXXXXXXX XXXXXXXX
	    // 11110XXX 10XXXXXX 10XXXXXX 10XXXXXX
	    wtmp = 0b11110000;
	    wtmp |= tmp >> 2;
	    out.push(wtmp);
		
	    wtmp = 0b10000000;
	    wtmp |= (tmp & 0b11) << 4;
	    tmp = cp.remove(0); // Fresh Byte
	    wtmp |= (tmp & 0b11110000) >> 4;
	    out.push(wtmp);

	    wtmp = 0b10000000;
	    wtmp |= (tmp & 0b1111) << 2;
	    tmp = cp.remove(0); // Fresh Byte
	    wtmp |= (tmp & 0b11000000) >> 6;
	    out.push(wtmp);

	    wtmp = 0b10000000;
	    wtmp |= tmp & 0b111111;
	    out.push(wtmp);
	}
	    
	_ => panic!("Invalid UTF-32 input."),
    }
}
