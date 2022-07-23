
use itertools::Itertools;
use std::fmt::Write;

pub const DEFAULT_SYMBOLS: [char; 10] = ['┼', '┤', '╶', '╴', '─', '╰', '╭', '╮', '╯', '│'];

#[derive(Debug)]
pub struct Config {
    pub symbols: [char; 10], // defaults to DEFAULT_SYMBOLS
    pub width: usize, // None for variate
    pub height: usize, // None for integer scheme; default = 5
    // what if w=0 or h=0?

    pub v_bot: f64,
    pub v_step: f64,

    pub label_bodywidth: usize,
    pub label_precision: usize,
}

pub fn plot(vss: &Vec<(Vec<f64>,u32)>, cfg: Config) -> String {
    // TODO check if v_step is positive

    let label_margin = {
        let abs_width = cfg.label_bodywidth + 1 + cfg.label_precision; // add 1 for midpoint
        // left space 1, neg sign 1/0, the number with ljust, right space 1
        1 + if cfg.v_bot < 0. {1} else {0} + abs_width + 1
    };

    let mut buffer = vec![vec![(' ', 9); label_margin + cfg.width]; cfg.height];
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
        let label = format!(
            "{number:LW$.PREC$} ",
            LW = cfg.label_bodywidth,
            PREC = cfg.label_precision,
            number = cfg.v_bot + (y as f64) * cfg.v_step,
        );
        let offset = label_margin - label.len(); // label is all ascii, 1 byte per char
        for (i,c) in label.chars().enumerate() {
            buffer[y][offset+i] = (c, 9);
        }
        buffer[y][label_margin] = (cfg.symbols[1], 9); // axis char
    }

    // FIXME what about single row?? what about inf values?
    //let clamp = |v| v.max(v_max + 1.).min(v_min - 1.)
    let scaled = |v :f64| (!v.is_nan()).then_some(((v-cfg.v_bot)/cfg.v_step).round() as usize);
    // what about INF?

    // margin + axis char 1
    let offset = label_margin + 1;

    for (vs,color) in vss {
        let color = *color;

        if let Some(&v) = vs.get(0) {
            if let Some(y) = scaled(v) { // what if INF or NAN?
                buffer[y][offset-1] = (cfg.symbols[0], color);
                // continued axis char
            }
        }

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

    let mut ret = String::new();
    for line in buffer.into_iter().rev() {
        for (chr, color) in line {
            if color == 9 || chr == ' ' {
                write!(ret, "{}", chr).unwrap();
            } else {
                write!(ret, "\x1b[3{}m{}\x1b[0m", color, chr).unwrap();
            }
        }
        write!(ret, "\n").unwrap();
    }

    ret
}


pub use clap::Parser;

#[derive(Parser, Debug, Default)]
#[clap(name = "asciichart-cui")]
#[clap(author, version, about, long_about = None)] // read from Cargo.toml
pub struct Args {

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

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    #[default] // FIXME
    Fast, Slow
}

fn validate_tileset(s :&str) -> Result<(), &'static str> {
    if s.len() == 10 {Ok(())} else {Err("should be of length 10")}
}

// ignore NaN & +/-INF
fn min_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v<a {v} else {a})
}
fn max_f64<T> (iter: T) -> Option<f64> where T: Iterator<Item=f64> {
    iter.filter(|v|!v.is_nan() && !v.is_infinite()).reduce(|a,v| if v>a {v} else {a})
}

impl Args {
  // handles generates configs, calculates defaults that are data-related
    pub fn gen_config(&self, vss: &Vec<(Vec<f64>,u32)>) -> Option<Config> {

        let (v_bot, v_top) = {
            let nan = std::f64::NAN;
            let min = min_f64(vss.iter().map(|(vs,_)| min_f64(vs.iter().cloned()).unwrap_or(nan)));
            let max = max_f64(vss.iter().map(|(vs,_)| max_f64(vs.iter().cloned()).unwrap_or(nan)));

            if let (Some(min), Some(max)) = (min, max) {
                (self.ymin.unwrap_or(min), self.ymax.unwrap_or(max))
            } else {
                return None;
            }
        };
        println!("min:{} max:{}", v_bot, v_top);

        let width = self.width.unwrap_or(vss.iter().map(|vs| vs.0.len()).min().unwrap_or(0));

        let v_interval = v_top - v_bot; // >= 0
        let height = if v_interval == 0. {1} else { // force height to 1 if single-valued
            self.height.unwrap_or(1 + v_interval.floor() as usize)
        }; // >= 1
        println!("width:{} height:{}", width, height);

        // use integer mode when height is not specified
        let v_step = if height == 1 {0.} else { v_interval / (height-1) as f64 };
        println!("intv:{} step:{}", v_interval, v_step);

        let label_bodywidth = 1 + v_top.abs().max(v_bot.abs()).log10().floor() as usize;
        let label_precision = {
            let signum = if v_step != 0. {v_step} else if v_bot != 0. {v_bot} else {1.};
            let prec = 1 - signum.log10().floor() as i32;
            (if self.height.is_none() {0} else {1}).max(prec) as usize
            // force prec >= 1 unless height=None (integer mode)
        };
        println!("<{}>.<{}>", label_bodywidth, label_precision);

        let mut cfg = Config {
            symbols: DEFAULT_SYMBOLS,
            v_bot: v_bot, v_step: v_step, width: width, height: height,
            label_bodywidth: label_bodywidth, label_precision: label_precision,
        };

        if let Some(tileset) = &self.tileset {
            for (i,c) in tileset.chars().enumerate() { cfg.symbols[i] = c; }
        }

        Some(cfg)
    }
}


#[cfg(test)]
mod tests {

    macro_rules! toF64 {
      (-) => { std::f64::NAN };
      (^) => { std::f64::INF };
      (v) => { std::f64::NINF };
      ($num:expr) => { f64::from($num) };
    }

    macro_rules! toSeries {
        ([$($series:tt),*]) => {
            vec![$(toF64!($series),)*]
        };
    }

    macro_rules! graph_eq {
        ($testname:ident ? $($key:ident = $val:expr),* ; $($series:tt),* => $rhs:expr) => {
          #[test]
          fn $testname(){
            let vss = vec![$((toSeries!($series),9),)*];
            #[allow(unused_mut)]
            let mut args = crate::Args::default();
            $(args.$key = $val;),*
            // FIXME use cfg.$key
            let cfg = args.gen_config(&vss).unwrap();
            let ret = crate::plot(&vss, cfg);
            let ref_line_start = if $rhs.chars().next() == Some('\n') {1} else {0};
            for (line1, line2) in std::iter::zip(ret.lines(), $rhs[ref_line_start..].lines()) {
              assert_eq!(line1.trim_end(), line2.trim_end());
            }
          }
        };
        ($testname:ident ? [$($series:expr),*]  ? $config:expr => $rhs:expr) => {
          #[test]
          fn $testname(){
            let res = plot(vec![$(f64::from($series),)*], $config);
            assert_eq!(res, $rhs);
          }
        };
    }

    // Missing data values in the series can be specified as a NaN.
    graph_eq!(nan_at_top ? height=Some(4) ; [1,2,3,4,-,4,3,2,1] => "
 4.0 ┤  ╭╴╶╮
 3.0 ┤ ╭╯  ╰╮
 2.0 ┤╭╯    ╰╮
 1.0 ┼╯      ╰ ");

    // `series` can also be a list of lists to support multiple data series.
    graph_eq!(mountain_valley ? height=Some(4) ;
              [10,20,30,40,30,20,10], [40,30,20,10,20,30,40] => "
 40.0 ┼╮ ╭╮ ╭
 30.0 ┤╰╮╯╰╭╯
 20.0 ┤╭╰╮╭╯╮
 10.0 ┼╯ ╰╯ ╰ ");

        /*
    `cfg` is an optional dictionary of various parameters to tune the appearance
    of the chart. `min` and `max` will clamp the y-axis and all values:
        >>> series = [1,2,3,4,float("nan"),4,3,2,1]
        >>> print(plot(series, {'min': 0}))
            4.00  ┼  ╭╴╶╮
            3.00  ┤ ╭╯  ╰╮
            2.00  ┤╭╯    ╰╮
            1.00  ┼╯      ╰
            0.00  ┤
        >>> print(plot(series, {'min': 2}))
            4.00  ┤  ╭╴╶╮
            3.00  ┤ ╭╯  ╰╮
            2.00  ┼─╯    ╰─
        >>> print(plot(series, {'min': 2, 'max': 3}))
            3.00  ┤ ╭─╴╶─╮
            2.00  ┼─╯    ╰─

            */

    // `height` specifies the number of rows the graph should occupy. It can be
    // used to scale down a graph with large data values:
    graph_eq!(mountain ? height=Some(5) ; [10,20,30,40,50,40,30,20,10] => "
 50.0 ┤   ╭╮
 40.0 ┤  ╭╯╰╮
 30.0 ┤ ╭╯  ╰╮
 20.0 ┤╭╯    ╰╮
 10.0 ┼╯      ╰ ");

    /*
    `format` specifies a Python format string used to format the labels on the
    y-axis. The default value is "{:8.2f} ". This can be used to remove the
    decimal point:
        >>> series = [10,20,30,40,50,40,30,20,10]
        >>> print(plot(series, {'height': 4, 'format':'{:8.0f}'}))
              50 ┤   ╭╮
              40 ┤  ╭╯╰╮
              30 ┤ ╭╯  ╰╮
              20 ┤╭╯    ╰╮
              10 ┼╯      ╰

              */
    graph_eq!(test_ones  ? ; [1, 1, 1, 1, 1] => " 1.0 ┼────");
    graph_eq!(test_ones_ ? height=Some(3) ; [1, 1, 1, 1, 1] => " 1.0 ┼────");
    graph_eq!(test_zeros ? ; [0, 0, 0, 0, 0] => " 0.0 ┼────");
    graph_eq!(test_zeros_? height=Some(3) ; [0, 0, 0, 0, 0] => " 0.0 ┼────");
}

    /*
    graph_eq!(test_three ? height=None ; [2,1,1,2,(-2),5,7,11,3,7,1] => "
 11.0 ┤      ╭╮
 10.0 ┤      ││
  9.0 ┤      ││
  8.0 ┤      ││
  7.0 ┤     ╭╯│╭╮
  6.0 ┤     │ │││
  5.0 ┤    ╭╯ │││
  4.0 ┤    │  │││
  3.0 ┤    │  ╰╯│
  2.0 ┼╮ ╭╮│    │
  1.0 ┤╰─╯││    ╰
  0.0 ┤   ││
 -1.0 ┤   ││
 -2.0 ┤   ╰╯     ");
}

    graph_eq!(test_four ? height=None ; [2,1,1,2,(-2),5,7,11,3,7,4,5,6,9,4,0,6,1,5,3,6,2] => "
 11.0 ┤      ╭╮
 10.0 ┤      ││
  9.0 ┼      ││    ╭╮
  8.0 ┤      ││    ││
  7.0 ┤     ╭╯│╭╮  ││
  6.0 ┤     │ │││ ╭╯│ ╭╮  ╭╮
  5.0 ┤    ╭╯ │││╭╯ │ ││╭╮││
  4.0 ┤    │  ││╰╯  ╰╮││││││
  3.0 ┤    │  ╰╯     ││││╰╯│
  2.0 ┼╮ ╭╮│         ││││  ╰
  1.0 ┤╰─╯││         ││╰╯
  0.0 ┤   ││         ╰╯
 -1.0 ┤   ││
 -2.0 ┤   ╰╯                 ");

    graph_eq!(test_five ? [ 2,1,1,2,-2,5,7,11,3,7,4,5,6,9,4,0,6,1,5,3,6,2] ?
                crate::Config::default().with_caption("Plot using asciigraph.".to_string())
     => " 11.00 ┤      ╭╮
 10.00 ┤      ││
  9.00 ┼      ││    ╭╮
  8.00 ┤      ││    ││
  7.00 ┤     ╭╯│╭╮  ││
  6.00 ┤     │ │││ ╭╯│ ╭╮  ╭╮
  5.00 ┤    ╭╯ │││╭╯ │ ││╭╮││
  4.00 ┤    │  ││╰╯  ╰╮││││││
  3.00 ┤    │  ╰╯     ││││╰╯│
  2.00 ┼╮ ╭╮│         ││││  ╰
  1.00 ┤╰─╯││         ││╰╯
  0.00 ┤   ││         ╰╯
 -1.00 ┤   ││
 -2.00 ┤   ╰╯
          Plot using asciigraph." );

    graph_eq!(test_six ? [0.2,0.1,0.2,2,-0.9,0.7,0.91,0.3,0.7,0.4,0.5] ?
    crate::Config::default().with_caption("Plot using asciigraph.".to_string())
    => "  2.00 ┤  ╭╮ ╭╮
  0.55 ┼──╯│╭╯╰───
 -0.90 ┤   ╰╯
          Plot using asciigraph." );

    graph_eq!(test_seven ? [2,1,1,2,-2,5,7,11,3,7,1] ?
    crate::Config::default().with_height(4).with_offset(3)
    => " 11.00 ┤      ╭╮
  7.75 ┼    ╭─╯│╭╮
  4.50 ┼╮ ╭╮│  ╰╯│
  1.25 ┤╰─╯││    ╰
 -2.00 ┤   ╰╯     "
    );

    graph_eq!(test_eight ? [0.453,0.141,0.951,0.251,0.223,0.581,0.771,0.191,0.393,0.617,0.478]
    => " 0.95 ┤ ╭╮
 0.85 ┤ ││  ╭╮
 0.75 ┤ ││  ││
 0.65 ┤ ││ ╭╯│ ╭╮
 0.55 ┤ ││ │ │ │╰
 0.44 ┼╮││ │ │╭╯
 0.34 ┤│││ │ ││
 0.24 ┤││╰─╯ ╰╯
 0.14 ┤╰╯        ");

    graph_eq!(test_nine ? [0.01, 0.004, 0.003, 0.0042, 0.0083, 0.0033, 0.0079]
    => " 0.010 ┼╮
 0.009 ┤│
 0.008 ┤│  ╭╮╭
 0.007 ┤│  │││
 0.006 ┤│  │││
 0.005 ┤│  │││
 0.004 ┤╰╮╭╯││
 0.003 ┤ ╰╯ ╰╯"
    );

    graph_eq!(test_ten ? [192,431,112,449,-122,375,782,123,911,1711,172] ? crate::Config::default().with_height(10)
    => " 1711 ┤        ╭╮
 1528 ┼        ││
 1344 ┤        ││
 1161 ┤        ││
  978 ┤       ╭╯│
  794 ┤     ╭╮│ │
  611 ┤     │││ │
  428 ┤╭╮╭╮╭╯││ │
  245 ┼╯╰╯││ ╰╯ ╰
   61 ┤   ││
 -122 ┤   ╰╯     ");

    graph_eq!(test_eleven ? [0.3189989805, 0.149949026, 0.30142492354, 0.195129182935, 0.3142492354,
    0.1674974513, 0.3142492354, 0.1474974513, 0.3047974513] ?
    crate::Config::default().with_width(30).with_height(5).with_caption("Plot with custom height & width.".to_string())
        => " 0.32 ┼╮            ╭─╮     ╭╮     ╭
 0.29 ┤╰╮    ╭─╮   ╭╯ │    ╭╯│     │
 0.26 ┤ │   ╭╯ ╰╮ ╭╯  ╰╮  ╭╯ ╰╮   ╭╯
 0.23 ┤ ╰╮ ╭╯   ╰╮│    ╰╮╭╯   ╰╮ ╭╯
 0.20 ┤  ╰╮│     ╰╯     ╰╯     │╭╯
 0.16 ┤   ╰╯                   ╰╯
         Plot with custom height & width."
    );

    graph_eq!(test_twelve ? [0,0,0,0,1.5,0,0,-0.5,9,-3,0,0,1,2,1,0,0,0,0,
				0,0,0,0,1.5,0,0,-0.5,8,-3,0,0,1,2,1,0,0,0,0,
				0,0,0,0,1.5,0,0,-0.5,10,-3,0,0,1,2,1,0,0,0,0] ?
                crate::Config::default().with_offset(10).with_height(10).with_caption("I'm a doctor, not an engineer.".to_string())
    => "     10.00    ┤                                             ╭╮
      8.70    ┤       ╭╮                                    ││
      7.40    ┼       ││                 ╭╮                 ││
      6.10    ┤       ││                 ││                 ││
      4.80    ┤       ││                 ││                 ││
      3.50    ┤       ││                 ││                 ││
      2.20    ┤       ││   ╭╮            ││   ╭╮            ││   ╭╮
      0.90    ┤   ╭╮  ││  ╭╯╰╮       ╭╮  ││  ╭╯╰╮       ╭╮  ││  ╭╯╰╮
     -0.40    ┼───╯╰──╯│╭─╯  ╰───────╯╰──╯│╭─╯  ╰───────╯╰──╯│╭─╯  ╰───
     -1.70    ┤        ││                 ││                 ││
     -3.00    ┤        ╰╯                 ╰╯                 ╰╯
                 I'm a doctor, not an engineer.");

    graph_eq!(test_thirteen ? [-5,-2,-3,-4,0,-5,-6,-7,-8,0,-9,-3,-5,-2,-9,-3,-1]
    => "  0.00 ┤   ╭╮   ╭╮
 -1.00 ┤   ││   ││     ╭
 -2.00 ┤╭╮ ││   ││  ╭╮ │
 -3.00 ┤│╰╮││   ││╭╮││╭╯
 -4.00 ┤│ ╰╯│   │││││││
 -5.00 ┼╯   ╰╮  │││╰╯││
 -6.00 ┤     ╰╮ │││  ││
 -7.00 ┤      ╰╮│││  ││
 -8.00 ┤       ╰╯││  ││
 -9.00 ┼         ╰╯  ╰╯ ");

    graph_eq!(test_fourteen ? [-0.000018527,-0.021,-0.00123,0.00000021312,
    -0.0434321234,-0.032413241234,0.0000234234] ?
        crate::Config::default().with_height(5).with_width(45)
        => "  0.000 ┼─╮           ╭────────╮                    ╭
 -0.008 ┤ ╰──╮     ╭──╯        ╰─╮                ╭─╯
 -0.017 ┤    ╰─────╯             ╰╮             ╭─╯
 -0.025 ┤                         ╰─╮         ╭─╯
 -0.034 ┤                           ╰╮   ╭────╯
 -0.042 ┼                            ╰───╯           "
    );

    graph_eq!(test_fifteen ? [57.76,54.04,56.31,57.02,59.5,52.63,52.97,56.44,56.75,52.96,55.54,
    55.09,58.22,56.85,60.61,59.62,59.73,59.93,56.3,54.69,55.32,54.03,50.98,50.48,54.55,47.49,
    55.3,46.74,46,45.8,49.6,48.83,47.64,46.61,54.72,42.77,50.3,42.79,41.84,44.19,43.36,45.62,
    45.09,44.95,50.36,47.21,47.77,52.04,47.46,44.19,47.22,45.55,40.65,39.64,37.26,40.71,42.15,
    36.45,39.14,36.62]
   => " 60.61 ┤             ╭╮ ╭╮
 59.60 ┤   ╭╮        │╰─╯│
 58.60 ┤   ││      ╭╮│   │
 57.59 ┼╮ ╭╯│      │││   │
 56.58 ┤│╭╯ │ ╭─╮  │╰╯   ╰╮
 55.58 ┤││  │ │ │╭─╯      │╭╮    ╭╮
 54.57 ┤╰╯  │ │ ││        ╰╯╰╮ ╭╮││      ╭╮
 53.56 ┤    │╭╯ ╰╯           │ ││││      ││
 52.56 ┤    ╰╯               │ ││││      ││           ╭╮
 51.55 ┤                     ╰╮││││      ││           ││
 50.54 ┤                      ╰╯│││      ││╭╮      ╭╮ ││
 49.54 ┤                        │││  ╭─╮ ││││      ││ ││
 48.53 ┤                        │││  │ │ ││││      ││ ││
 47.52 ┤                        ╰╯│  │ ╰╮││││      │╰─╯╰╮╭╮
 46.52 ┤                          ╰─╮│  ╰╯│││      │    │││
 45.51 ┤                            ╰╯    │││   ╭──╯    ││╰╮
 44.50 ┤                                  │││ ╭╮│       ╰╯ │
 43.50 ┤                                  ││╰╮│╰╯          │
 42.49 ┤                                  ╰╯ ╰╯            │   ╭╮
 41.48 ┤                                                   │   ││
 40.48 ┤                                                   ╰╮ ╭╯│
 39.47 ┤                                                    ╰╮│ │╭╮
 38.46 ┤                                                     ││ │││
 37.46 ┤                                                     ╰╯ │││
 36.45 ┤                                                        ╰╯╰"
    );

    #[test]
    fn test_min_max() {
        assert_eq!(
            (-2f64, 11f64),
            crate::min_max(&vec![
                2f64, 1f64, 1f64, 2f64, -2f64, 5f64, 7f64, 11f64, 3f64, 7f64, 1f64
            ])
        );
    }

}

*/
