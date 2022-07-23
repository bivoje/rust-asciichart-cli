
use rasciigraph::{plot};
use clap::{Parser};
use itertools::Itertools;

#[derive(Parser, Debug)]
#[clap(name = "asciichart-cui")]
#[clap(author, version, about, long_about = None)] // read from Cargo.toml
struct Args {
    #[clap(short, long, value_parser, default_value_t=0)]
    width: u32,

    #[clap(short, long, value_parser, default_value_t=0)]
    height: u32,

    #[clap(short, long, value_parser, arg_enum, default_value_t=Mode::Fast)]
    // FIXME arg_enum attribute makes clap don't require Display implementation on Mode
    // But there's not documentation for that... :/
    // https://github.com/clap-rs/clap/issues/3185
    // https://github.com/clap-rs/clap/pull/3188
    mode: Mode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
enum Mode {
    Fast, Slow
}

fn main() {
    let _args = Args::parse();

    let width = 80;
    let mut v = vec![0f64; width];
    for i in 0 .. width {
        let pi = std::f64::consts::PI;
        //v[i] = 3000. +  0.000001 * (i as f64 * pi * 4.0 / 120.0).sin();
        v[i] = 100. + 0.01 * (i as f64 * pi * 4.0 / width as f64).sin();
    }

    //let _ret = asciichart_plot(vec![(vec![1.],3)], Config {symbols: DEFAULT_SYMBOLS, width: None, height: None});
    let _ret = asciichart_plot(&vec![(v.clone(),1)], Config {
        symbols: DEFAULT_SYMBOLS, width: None, height: Some(9)
    });

    println!( "{}", plot(v,
        rasciigraph::Config::default().with_height(8)
    ));
}

const DEFAULT_SYMBOLS: [char; 10] = ['┼', '┤', '╶', '╴', '─', '╰', '╭', '╮', '╯', '│'];

struct Config {
    symbols: [char; 10], // defaults to DEFAULT_SYMBOLS
    width: Option<usize>, // None for variate
    height: Option<usize>, // None for integer scheme; default = 5
    // what if w=0 or h=0?
}


// ignore NaN & +/-INF
fn min_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v<a {v} else {a})
}
fn max_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v>a {v} else {a})
}

//if vs.is_empty() {iter::once(f32::)}
fn asciichart_plot(vss: &Vec<(Vec<f64>,u32)>, cfg: Config) -> Result<String, &'static str> {

    let (v_min, v_max) = {
        let nan = std::f64::NAN;
        let min = min_f64(vss.iter().map(|(vs,_)| min_f64(vs.iter().cloned()).unwrap_or(nan)));
        let max = max_f64(vss.iter().map(|(vs,_)| max_f64(vs.iter().cloned()).unwrap_or(nan)));

        if let (Some(min), Some(max)) = (min, max) {
            (min, max)
        } else {
            return Err("empty graph");
        }
    };
    println!("min:{} max:{}", v_min, v_max);

    let v_interval = v_max - v_min; // >= 0
    let height = cfg.height.unwrap_or(1 + v_interval.floor() as usize); // >= 1
    // use integer mode when height is not specified
    let v_step = if height == 1 {0.} else { v_interval / (height-1) as f64 };
    println!("intv:{} h:{} step:{}", v_interval, height, v_step);

    let label_precision = {
        let prec = 1 - v_step.log10().floor() as i32;
        (if cfg.height.is_none() {0} else {1}).max(prec) as usize
        // force prec >= 1 unless height=None (integer mode)
    };
    let label_bodywidth = 1 + v_max.abs().max(v_min.abs()).log10().floor() as usize;
    // FIXME what if 0???
    println!("<{}>.<{}>", label_bodywidth, label_precision);

    let width = cfg.width.unwrap_or(vss.iter().map(|vs| vs.0.len()).min().unwrap_or(0));

    let label_margin = {
        let abs_width = label_bodywidth + 1 + label_precision; // add 1 for midpoint
        // left space 1, neg sign 1/0, the number with ljust, right space 1
        1 + if v_min < 0. {1} else {0} + abs_width + 1
    };
    let mut buffer = vec![vec![(' ', 9); label_margin + width]; 1+height];
    println!("width:{} lm:{}", width, label_margin);

    for y in 0..height { // never executed when height == 1, in which case min==max
        // String uses Vec[u8] in specified (default UTF-8) encoding internally,
        // while hiding the internals to outer codes.
        // write_* functions produces u8 stream,
        // which then can be written to String's internal buffer.
        // but Vec<char> does not supported (no write_fmt implementation..)
        // format! creates temporary String,
        // this is wasteful yet I couldn't find way to get char stream from format!.
        // https://stackoverflow.com/a/24542502
        let number = v_min + (y as f64) * v_step;
        let mut label = format!(
            "{number:LW$.PREC$} ",
            LW = label_bodywidth,
            PREC = label_precision,
            number = number,
        );
        let offset = label_margin - label.len(); // label is all ascii, 1 byte per char
        label.push(if number == 0. {cfg.symbols[0]} else {cfg.symbols[1]});
        for (i,c) in label.chars().enumerate() {
            buffer[y][offset+i] = (c, 9);
        }
    }

    // FIXME what about single row?? what about inf values?
    //let clamp = |v| v.max(v_max + 1.).min(v_min - 1.)
    let scaled = |v :f64| (!v.is_nan()).then_some(((v-v_min)/v_step).round() as usize);
    // what about INF?

    let offset = label_margin + 1;
    for (vs,color) in vss {
        let color = *color;
        // FIXME use .take(n)
        let vvs = vs[..width.min(vs.len())].into_iter().cloned().tuple_windows();
        for (x,(v0,v1)) in vvs.enumerate() {

            match (scaled(v0), scaled(v1)) {
                (None, None) => continue,
                (None, Some(y)) =>
                    buffer[y][x+offset] = (cfg.symbols[2], color),
                (Some(y), None) =>
                    buffer[y][x+offset] = (cfg.symbols[3], color),
                (Some(y0), Some(y1)) if y0 == y1 =>
                    buffer[y0][x+offset] = (cfg.symbols[4], color),
                (Some(y0), Some(y1)) => {
                    buffer[y1][x+offset] = (if y0 > y1 {cfg.symbols[5]} else {cfg.symbols[6]}, color);
                    buffer[y0][x+offset] = (if y0 > y1 {cfg.symbols[7]} else {cfg.symbols[8]}, color);

                    for y in y0.min(y1)+1 ..= y0.max(y1)-1 { // FIXME what if single row?
                        buffer[y][x+offset] = (cfg.symbols[9], color)
                    }
                },
            }
        }
    }

    use std::fmt::Write;
    let mut ret = String::new();
    for line in buffer.into_iter().rev() {
        for (chr, color) in line {
            if color == 9 || chr == ' ' {
                write!(ret, "{}", chr).unwrap();
            } else {
                write!(ret, "\x1b[{}m{}\x1b[0m", color, chr).unwrap();
            }
        }
        write!(ret, "\n").unwrap();
    }

    print!("{}", ret);
    Ok(ret)
}
