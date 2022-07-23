
use clap::Parser;
use itertools::Itertools;

#[derive(Parser, Debug)]
#[clap(name = "asciichart-cui")]
#[clap(author, version, about, long_about = None)] // read from Cargo.toml
struct Args {

    #[clap(long, value_parser)]
    ymax: Option<f64>,

    #[clap(long, value_parser)]
    ymin: Option<f64>,

    #[clap(short, long, value_parser)]
    width: Option<usize>,

    #[clap(short, long, value_parser)]
    height: Option<usize>,

    #[clap(long, value_parser, validator=validate_tileset, arg_enum)]
    tileset: Option<String>,

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

fn validate_tileset(s :&str) -> Result<(), &'static str> {
    if s.len() == 10 {Ok(())} else {Err("should be of length 10")}
}

fn main() {
    let args = Args::parse();

    let width = 80;
    let mut v = vec![0f64; width];
    for i in 0 .. width {
        let pi = std::f64::consts::PI;
        //v[i] = 3000. +  0.000001 * (i as f64 * pi * 4.0 / 120.0).sin();
        v[i] = 100. + 0.01 * (i as f64 * pi * 4.0 / width as f64).sin();
    }
    let vss = vec![(v,1)];

    let cfg = gen_config(&vss, args).unwrap(); // FIXME
    let _ret = plot(&vss, cfg);

    println!( "{}", rasciigraph::plot(vss[0].0.clone(),
        rasciigraph::Config::default().with_height(8)
    ));
}

const DEFAULT_SYMBOLS: [char; 10] = ['┼', '┤', '╶', '╴', '─', '╰', '╭', '╮', '╯', '│'];

#[derive(Debug, Default)]
struct Config {
    symbols: [char; 10], // defaults to DEFAULT_SYMBOLS
    width: usize, // None for variate
    height: usize, // None for integer scheme; default = 5
    // what if w=0 or h=0?

    v_bot: f64,
    v_top: f64,
    v_step: f64,

    label_bodywidth: usize,
    label_precision: usize,
}


// ignore NaN & +/-INF
fn min_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v<a {v} else {a})
}
fn max_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v>a {v} else {a})
}

// handles generates configs, calculates defaults that are data-related
fn gen_config(vss: &Vec<(Vec<f64>,u32)>, args: Args) -> Option<Config> {

    let (v_bot, v_top) = {
        let nan = std::f64::NAN;
        let min = min_f64(vss.iter().map(|(vs,_)| min_f64(vs.iter().cloned()).unwrap_or(nan)));
        let max = max_f64(vss.iter().map(|(vs,_)| max_f64(vs.iter().cloned()).unwrap_or(nan)));

        if let (Some(min), Some(max)) = (min, max) {
            (args.ymin.unwrap_or(min), args.ymax.unwrap_or(max))
        } else {
            return None;
        }
    };
    println!("min:{} max:{}", v_bot, v_top);

    let width = args.width.unwrap_or(vss.iter().map(|vs| vs.0.len()).min().unwrap_or(0));

    let v_interval = v_top - v_bot; // >= 0
    let height = args.height.unwrap_or(1 + v_interval.floor() as usize); // >= 1
    println!("width:{} height:{}", width, height);

    // use integer mode when height is not specified
    let v_step = if height == 1 {0.} else { v_interval / (height-1) as f64 };
    println!("intv:{} h:{} step:{}", v_interval, height, v_step);

    let label_bodywidth = 1 + v_top.abs().max(v_bot.abs()).log10().floor() as usize;
    let label_precision = {
        let prec = 1 - v_step.log10().floor() as i32;
        (if args.height.is_none() {0} else {1}).max(prec) as usize
        // force prec >= 1 unless height=None (integer mode)
    };
    // FIXME what if 0???
    println!("<{}>.<{}>", label_bodywidth, label_precision);

    let mut cfg = Config {
        symbols: DEFAULT_SYMBOLS,
        v_bot: v_bot, v_top: v_top, v_step: v_step, width: width, height: height,
        label_bodywidth: label_bodywidth, label_precision: label_precision,
    };

    if let Some(tileset) = args.tileset {
        for (i,c) in tileset.chars().enumerate() { cfg.symbols[i] = c; }
    }

    Some(cfg)
}

fn plot(vss: &Vec<(Vec<f64>,u32)>, cfg: Config) -> String {
    // TODO check if v_step is positive

    let label_margin = {
        let abs_width = cfg.label_bodywidth + 1 + cfg.label_precision; // add 1 for midpoint
        // left space 1, neg sign 1/0, the number with ljust, right space 1
        1 + if cfg.v_bot < 0. {1} else {0} + abs_width + 1
    };

    let mut buffer = vec![vec![(' ', 9); label_margin + cfg.width]; 1+cfg.height];
    println!("width:{} lm:{}", cfg.width, label_margin);

    for y in 0..cfg.height { // never executed when height == 1, in which case min==max
        // String uses Vec[u8] in specified (default UTF-8) encoding internally,
        // while hiding the internals to outer codes.
        // write_* functions produces u8 stream,
        // which then can be written to String's internal buffer.
        // but Vec<char> does not supported (no write_fmt implementation..)
        // format! creates temporary String,
        // this is wasteful yet I couldn't find way to get char stream from format!.
        // https://stackoverflow.com/a/24542502
        let number = cfg.v_bot + (y as f64) * cfg.v_step;
        let mut label = format!(
            "{number:LW$.PREC$} ",
            LW = cfg.label_bodywidth,
            PREC = cfg.label_precision,
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
    let scaled = |v :f64| (!v.is_nan()).then_some(((v-cfg.v_bot)/cfg.v_step).round() as usize);
    // what about INF?

    // margin + axis char 1
    let offset = label_margin + 1;

    for (vs,color) in vss {
        let color = *color;
        // FIXME use .take(n)
        let vvs = vs[..cfg.width.min(vs.len())].into_iter().cloned().tuple_windows();
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
    ret
}
