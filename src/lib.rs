
use itertools::Itertools;
use std::fmt::Write;
use std::collections::VecDeque;

pub const UNICODE_SYMBOLS: [char; 13] = ['┼','┤','╶','╴','─','╰' ,'╭','╮','╯','│','╞','═','╤'];
pub const   ASCII_SYMBOLS: [char; 13] = ['L','I','<','>','_','\\','.','.','/','|','v','-','v'];

#[derive(Debug)]
pub struct Config {
    pub symbols: [char; 13],
    pub width: usize, // TODO TEST None for variate
    // what if w=0 or h=0?

    pub label_bot: f64,
    pub label_top: f64,
    pub v_step: f64,

    pub label_precision: usize,

    // x_start, x_step, x_prec, x_interval
    pub x_label: Option<(f64,f64,usize,usize)>,
}

// TODO flowing x label when monitoring?

pub fn plot(vss: &Vec<(VecDeque<f64>,u32)>, cfg: Config) -> (String, usize) {
    assert!(cfg.label_bot <= cfg.label_top);
    assert!(cfg.v_step >= 0.); // TODO v_step < 0 && label_bot > label_top for inverted??
    assert!(cfg.x_label.filter(|x_label| x_label.3 == 0).is_none());

    let v_step = if cfg.v_step == 0. {f64::MIN_POSITIVE} else {cfg.v_step};
    // keep the value positive

    let height = {
        let intv = cfg.label_top - cfg.label_bot;
        if intv == 0. {1} else {1 + (intv/v_step).round() as usize}
    };

    let label_margin = {
        let label_bodywidth = {
            // FIXME WHAT IF label_top == label_bot == 0. ??
            let bot_width = 1 + if cfg.label_bot < 0. {1} else {0} + cfg.label_bot.abs().log10().floor() as usize;
            let top_width = 1 + if cfg.label_top < 0. {1} else {0} + cfg.label_top.abs().log10().floor() as usize;
            bot_width.max(top_width)
        };
        let abs_width = label_bodywidth + // add 1 for midpoint if precision is not 0
            if cfg.label_precision == 0 {0} else { 1 + cfg.label_precision };
        // left space 1, the number with ljust, right space 1
        1 + abs_width + 1
    };

    // note that each row had length `label_margin + cfg.width`, omitting 1 for mid-axis character.
    // this is because, the axis point is used to represent first data point.
    let mut buffer = vec![vec![(' ', 9); label_margin + cfg.width]; height];

    for y in 0..height {
        let label = format!(
            "{number:LW$.PREC$} ",
            LW = label_margin - 1, // subtract 1 for the trailing space
            PREC = cfg.label_precision,
            number = if y == height-1 {
                cfg.label_top // to avoid top label being like 1.9999999 for float error
            } else {
                cfg.label_bot + (y as f64) * v_step
            },
        );
        for (i,c) in label.chars().enumerate() { buffer[y][i] = (c, 9); }
        buffer[y][label_margin] = (cfg.symbols[1], 9); // '┤' axis char
    }

    // scale the value into row index. `-1` if too low, `height` if too high, `None` if NaN
    let scaled = |v :f64| (!v.is_nan()).then_some(
        if v < cfg.label_bot - v_step/2. { -1i32 }
        else if cfg.label_top + v_step/2. < v { height as i32 }
        else if v_step != 0. { ((v-cfg.label_bot)/v_step).round() as i32 }
        else {0}
    );

    // margin + axis char 1
    let offset = label_margin + 1;

    for (vs,color) in vss {

        let vvs = vs.iter().cloned().take(cfg.width).tuple_windows();
        for (x,(v0,v1)) in vvs.enumerate() { // runs at most width-1 times

            let mut put = |y, x, chr| if let Ok(y) = usize::try_from(y) {
                if y < height {
                    buffer[y as usize][x+offset] = (chr, *color);
                }
            };

            match (scaled(v0), scaled(v1)) {
                (None, None) => continue,
                (None, Some(y)) =>
                    put(y, x, cfg.symbols[2]), // '╶'
                (Some(y), None) =>
                    put(y, x, cfg.symbols[3]), // '╴'
                (Some(y0), Some(y1)) if y0 == y1 =>
                    put(y0, x, cfg.symbols[4]), // '─'
                (Some(y0), Some(y1)) => {
                    put(y1, x, if y0 > y1 {cfg.symbols[5]} else {cfg.symbols[6]}); // '╰', '╭'
                    put(y0, x, if y0 > y1 {cfg.symbols[7]} else {cfg.symbols[8]}); // '╮', '╯'

                    for y in y0.min(y1)+1 ..= y0.max(y1)-1 {
                        put(y, x, cfg.symbols[9]); // '│'
                    }
                },
            }
        }

        // for first valut, mark it on the vertical axis (continued axis)
        if let Some(&v) = vs.get(0) { if let Some(y) = scaled(v) {
            if 0 <= y && y < height as i32 {
                buffer[y as usize][offset-1] = (cfg.symbols[0], *color); // '┼' continued axis char
            }
        }}

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

    if let Some((x_start,x_step,x_prec,x_intv)) = cfg.x_label {
        // x-axis
        write!(ret, "{: ^1$}", "", offset-1).unwrap();
        let mut form = std::iter::repeat(cfg.symbols[11]).take(x_intv-1).collect::<String>();
        form.push(cfg.symbols[12]);
        let s = std::iter::repeat_with(||form.chars()).flatten().take(cfg.width).collect::<String>();
        write!(ret, "{}{}\n", cfg.symbols[10], s).unwrap();

        // x-labels
        write!(ret, "{: ^1$}", "", offset-1).unwrap();
        for i in (0..=cfg.width).step_by(x_intv) {
            write!(ret, "{:<1$.2$}", x_start + x_step * i as f64, x_intv, x_prec).unwrap();
        }
        write!(ret, "\n").unwrap();
    }

    (ret, height + if cfg.x_label.is_none() {0} else {3}) // FIXME why 3 not 2???
}


pub use clap::Parser;

// TODO color as options?
// TODO label on righthand?
// TODO multiple labels?? <- multiple min-max?
// TODO multiple plots?
#[derive(Parser, Debug, Default)]
#[clap(name = "asciichart-cui")]
#[clap(author, version, about, long_about = None)] // read from Cargo.toml
pub struct Args {

    /// # of digits after floating point for each x label.
    #[clap(long, value_parser)]
    pub xprec: Option<usize>,

    /// amount to increase x labels with. if omitted, x labels are not drawn.
    #[clap(long, value_parser)]
    pub xstep: Option<f64>,

    /// value for the first x label
    #[clap(long, value_parser, default_value_t=0.)]
    pub xmin: f64,


    /// # of digits after floating point for each y label.
    #[clap(short='p', long, value_parser)]
    pub yprec: Option<usize>,

    /// Maximum value of the vertical label.
    #[clap(short='M', long, value_parser)]
    pub ymax: Option<f64>,

    /// Minimum value of the vertical label.
    #[clap(short='m', long, value_parser)]
    pub ymin: Option<f64>,


    /// # of datapoints to plot, trailing data will be ignored.
    #[clap(short, long, value_parser)]
    pub width: Option<usize>,
    // TODO used as an argument to interpolate feature in the future

    /// # of rows in the plot. if not specified, height will be adjusted for integer-ranged labels.
    #[clap(short, long, value_parser)]
    pub height: Option<usize>,


    /// Characters to be used for plot. Must be a string of width 10,
    /// where each characters are used for
    /// 0: vertical axis continued, 1: vertical axis
    /// 2: horizontal left half, 3: horizontal right half, 4: horizontal whole,
    /// 5: L shape corner, 6: r shape corner, 7: flipped r shape corner, 8: j shape corner
    /// 9: vertical line.
    #[clap(long, value_parser, validator=validate_tileset, arg_enum)]
    pub tileset: Option<String>,

    /// Overwrite <TILESET> with ascii characters "LI<>_\\../|".
    #[clap(long, value_parser, default_value_t=false)]
    pub ascii: bool,


    /// Repeat drawing the plot for each datarow.
    #[clap(long, value_parser, default_value_t=false)]
    pub monitor: bool,

    /// Use specified demo data instead of reading from stdin.
    /// Possible values are "sincos", "rand" and "rand4".
    #[clap(long, value_parser)]
    pub demo: Option<String>,
}

fn validate_tileset(s :&str) -> Result<(), &'static str> {
    if s.len() == 13 {Ok(())} else {Err("should be of length 10")}
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
    pub fn gen_config(&self, vss: &Vec<(VecDeque<f64>,u32)>) -> Option<Config> {
      // FIXME is optional needed?

        let (v_bot, v_top) = {
            let nan = f64::NAN;
            let min = min_f64(vss.iter().map(|(vs,_)| min_f64(vs.iter().cloned()).unwrap_or(nan)));
            let max = max_f64(vss.iter().map(|(vs,_)| max_f64(vs.iter().cloned()).unwrap_or(nan)));

            if let (Some(min), Some(max)) = (min, max) {
                (self.ymin.unwrap_or(min), self.ymax.unwrap_or(max))
            } else {
                return None;
            }
        };

        let width = self.width.unwrap_or(vss.iter().map(|vs| vs.0.len()).min().unwrap_or(0));

        let v_interval = v_top - v_bot; // >= 0
        let height = if v_interval == 0. {1} else { // force height to 1 if single-valued
            self.height.unwrap_or(1 + v_interval.floor() as usize)
        }; // >= 1

        let (label_bot, label_top, v_step) = if height == 1 {
            // if user forced height=1, we need to make label_bot/top accordingly
            let mid = (v_bot + v_top) / 2.;
            // to indicate the range of values coverd in the plot
            let v_step = v_interval * 1.5; // *1.5 for generouse error range
            (mid, mid, v_step)
        } else if self.height.is_none() {
            // use integer mode when height is not specified
            (v_bot.floor(), v_top.ceil(), 1.)
        } else {
            (v_bot, v_top, v_interval / (height-1) as f64)
        };

        let label_precision = self.yprec.unwrap_or({
            let signum = if v_step != 0. {v_step} else if label_bot != 0. {label_bot} else {1.};
            let prec = 1 - signum.log10().floor() as i32;
            (if self.height.is_none() {0} else {1}).max(prec) as usize
            // force prec >= 1 unless height=None (integer mode)
        });

        // x_start, x_step, x_prec, x_interval
        let x_label = self.xstep.map(|xstep| {
            let xprec = self.xprec.unwrap_or({
                let signum = if xstep != 0. {xstep} else {xstep.abs()};
                0f64.max(-signum.log10().floor()) as usize
            });
            let xint = {
                let body = (self.xmin + 10000.*xstep).abs().log10().ceil() as usize;
                println!("{} {}", body, xprec);
                2 * (body + 1 + xprec)
            };
            (self.xmin, xstep, xprec, xint)
        });

        let mut cfg = Config {
            symbols: if self.ascii {ASCII_SYMBOLS} else {UNICODE_SYMBOLS}, width: width,
            label_bot: label_bot, label_top: label_top, v_step: v_step,
            label_precision: label_precision, x_label: x_label,
        };

        if ! self.ascii {
            if let Some(tileset) = &self.tileset {
                for (i,c) in tileset.chars().enumerate() { cfg.symbols[i] = c; }
            }
        }

        Some(cfg)
    }
}


#[cfg(test)]
mod tests {
    use std::panic;
    use std::panic::panic_any;

    macro_rules! toF64 {
      (_) => { f64::NAN };
      (^) => { f64::INFINITY };
      (v) => { f64::NEG_INFINITY };
      ($num:expr) => { f64::from($num) };
    }

    macro_rules! toSeries {
        ([$($v:tt),*]) => {
            vec![$(toF64!($v),)*]
        };
    }

    macro_rules! set_arg {
        (cfg, $arg:ident, $key:ident, $val:expr) => {
        };
        (arg, $arg:ident, $key:ident, $val:expr) => {
            $arg.$key = Some($val);
        };
    }

    macro_rules! set_cfg {
        (cfg, $cfg:ident, $key:ident, $val:expr) => {
            $cfg.$key = Some($val);
        };
        (arg, $cfg:ident, $key:ident, $val:expr) => {
        };
    }

    macro_rules! graph_eq {
      ($testname:ident ? $($ctn:ident.$key:ident = $val:expr),* ; $($series:tt),* => $rhs:expr) => {
        #[test]
        fn $testname() {
          let vss = vec![$((toSeries!($series),9),)*];
          #[allow(unused_mut)]
          let mut arg = crate::Args::default();
          $(set_arg!($ctn, arg, $key, $val);)*
          #[allow(unused_mut)]
          let mut cfg = arg.gen_config(&vss).unwrap();
          $(set_cfg!($ctn, cfg, $key, $val);)*
          let ret = crate::plot(&vss, cfg);
          let ref_line_start = if $rhs.chars().next() == Some('\n') {1} else {0};
          for (line1, line2) in std::iter::zip(ret.lines(), $rhs[ref_line_start..].lines()) {
            let result = panic::catch_unwind(|| { // this works like try: clause
              assert_eq!(line1.trim_end(), line2.trim_end());
            });
            if result.is_err() { // this works like catch clause
              // report whole shape when mismatch occures
              print!("{}", ret);
              print!("{}", $rhs);
              panic_any(result.unwrap_err()); // re-raise
            }
          }
        }
      };
    }

    // test cases borrowed from
    // https://github.com/kroitor/asciichart/blob/master/asciichartpy/__init__.py

    // Missing data values in the series can be specified as a NaN.
    graph_eq!(nan_at_top ? arg.height=4 ; [1,2,3,4,_,4,3,2,1] => "
 4.0 ┤  ╭╴╶╮
 3.0 ┤ ╭╯  ╰╮
 2.0 ┤╭╯    ╰╮
 1.0 ┼╯      ╰ ");

    // `series` can also be a list of lists to support multiple data series.
    graph_eq!(mountain_valley ? arg.height=4 ;
              [10,20,30,40,30,20,10], [40,30,20,10,20,30,40] => "
 40.0 ┼╮ ╭╮ ╭
 30.0 ┤╰╮╯╰╭╯
 20.0 ┤╭╰╮╭╯╮
 10.0 ┼╯ ╰╯ ╰ ");

    // `cfg` is an optional dictionary of various parameters to tune the appearance
    // of the chart. `min` and `max` will clamp the y-axis and all values:
    graph_eq!(ymin0 ? arg.ymin=0. ; [1,2,3,4,_,4,3,2,1] => "
 4.0 ┤  ╭╴╶╮
 3.0 ┤ ╭╯  ╰╮
 2.0 ┤╭╯    ╰╮
 1.0 ┼╯      ╰
 0.0 ┤         ");

    graph_eq!(ymin1 ? arg.ymin=2. ; [1,2,3,4,_,4,3,2,1] => "
 4.0 ┤  ╭╴╶╮
 3.0 ┤ ╭╯  ╰╮
 2.0 ┤╭╯    ╰╮ ");

    graph_eq!(ymin2 ? arg.ymin=2., arg.ymax=3. ; [1,2,3,4,_,4,3,2,1] => "
 3.0 ┤ ╭╯  ╰╮
 2.0 ┤╭╯    ╰╮ ");

    // `height` specifies the number of rows the graph should occupy. It can be
    // used to scale down a graph with large data values:
    graph_eq!(mountain ? arg.height=5 ; [10,20,30,40,50,40,30,20,10] => "
 50.0 ┤   ╭╮
 40.0 ┤  ╭╯╰╮
 30.0 ┤ ╭╯  ╰╮
 20.0 ┤╭╯    ╰╮
 10.0 ┼╯      ╰ ");

    // `format` specifies a Python format string used to format the labels on the
    // y-axis. The default value is "{:8.2f} ". This can be used to remove the
    // decimal point:
    graph_eq!(precision ? arg.yprec=0, arg.height=5 ;
        [10,20,30,40,50,40,30,20,10] => "
 50 ┤   ╭╮
 40 ┤  ╭╯╰╮
 30 ┤ ╭╯  ╰╮
 20 ┤╭╯    ╰╮
 10 ┼╯      ╰ ");

    graph_eq!(test_ones  ? ; [1, 1, 1, 1, 1] => " 1.0 ┼────");
    graph_eq!(test_ones_ ? arg.height=3 ; [1, 1, 1, 1, 1] => " 1.0 ┼────");
    graph_eq!(test_zeros ? ; [0, 0, 0, 0, 0] => " 0.0 ┼────");
    graph_eq!(test_zeros_? arg.height=3 ; [0, 0, 0, 0, 0] => " 0.0 ┼────");

    graph_eq!(test_ones_jitter ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, 0.9999998, 1.0000012, 1] => " 1.0 ┼────");
    graph_eq!(test_onenans_jitter ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, _,         1.0000012, 1] => " 1.0 ┼─╴╶─");
    graph_eq!(test_oneinfs_jitter ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, ^,         1.0000012, 1] => " 1.0 ┼─╯╰─");
    graph_eq!(test_oneninfs_jitter ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, v,         1.0000012, 1] => " 1.0 ┼─╮╭─");
    graph_eq!(test_oneinfs_jittera ? arg.height=1, arg.yprec=1 ;
          [^,         1.000001, _,         1.0000012, 1] => " 1.0 ┤╰╴╶─");
    graph_eq!(test_oneninfs_jittera ? arg.height=1, arg.yprec=1 ;
          [v,         1.000001, _,         1.0000012, 1] => " 1.0 ┤╭╴╶─");
    graph_eq!(test_oneinfs_jitterb ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, _,         1.0000012, ^] => " 1.0 ┼─╴╶╯");
    graph_eq!(test_oneninfs_jitterb ? arg.height=1, arg.yprec=1 ;
          [0.9999999, 1.000001, _,         1.0000012, v] => " 1.0 ┼─╴╶╮");

    graph_eq!(test_three ? ; [2,1,1,2,(-2),5,7,11,3,7,1] => "
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

    graph_eq!(test_four ? ; [2,1,1,2,(-2),5,7,11,3,7,4,5,6,9,4,0,6,1,5,3,6,2] => "
 11.0 ┤      ╭╮
 10.0 ┤      ││
  9.0 ┤      ││    ╭╮
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

    graph_eq!(test_five ? ; [2,1,1,2,(-2),5,7,11,3,7,4,5,6,9,4,0,6,1,5,3,6,2] => "
 11.0 ┤      ╭╮
 10.0 ┤      ││
  9.0 ┤      ││    ╭╮
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

    graph_eq!(test_six ? arg.yprec = 2 ; [0.2,0.1,0.2,2,(-0.9),0.7,1.28,0.3,0.7,0.4,0.5] => "
  2.00 ┤  ╭╮ ╭╮
  0.55 ┼──╯│╭╯╰───
 -0.90 ┤   ╰╯      ");

    graph_eq!(test_seven ? arg.height=5, arg.yprec=2; [3,1,1,3,(-2),5,7,11,3,7,1] => "
 11.00 ┤      ╭╮
  7.75 ┤     ╭╯│╭╮
  4.50 ┼╮ ╭╮╭╯ ╰╯│
  1.25 ┤╰─╯││    ╰
 -2.00 ┤   ╰╯     ");

    graph_eq!(test_eight ? arg.height=9 ; [0.453,0.141,0.951,0.251,0.223,0.581,0.771,0.191,0.393,0.617,0.478] => "
 0.95 ┤ ╭╮
 0.85 ┤ ││
 0.75 ┤ ││  ╭╮
 0.65 ┤ ││  ││ ╭╮
 0.55 ┤ ││ ╭╯│ ││
 0.44 ┼╮││ │ │ │╰
 0.34 ┤│││ │ │╭╯
 0.24 ┤││╰─╯ ││
 0.14 ┤╰╯    ╰╯   ");

    graph_eq!(test_nine ? arg.height=8, arg.yprec=3;
        [0.01, 0.004, 0.003, 0.0042, 0.0083, 0.0033, 0.0079] => "
 0.010 ┼╮
 0.009 ┤│
 0.008 ┤│  ╭╮╭
 0.007 ┤│  │││
 0.006 ┤│  │││
 0.005 ┤│  │││
 0.004 ┤╰╮╭╯││
 0.003 ┤ ╰╯ ╰╯ ");

    graph_eq!(test_ten ? arg.height=11, arg.yprec=0;
        [192,431,112,449,(-122),375,782,123,911,1711,172] => "
 1711 ┤        ╭╮
 1528 ┤        ││
 1344 ┤        ││
 1161 ┤        ││
  978 ┤       ╭╯│
  794 ┤     ╭╮│ │
  611 ┤     │││ │
  428 ┤╭╮╭╮╭╯││ │
  245 ┼╯││││ ││ ╰
   61 ┤ ╰╯││ ╰╯
 -122 ┤   ╰╯     ");

    /* TODO interperse feature
    graph_eq!(test_eleven ? height = 5 ; [
        0.3189989805, 0.149949026, 0.30142492354, 0.195129182935, 0.3142492354,
        0.1674974513, 0.3142492354, 0.1474974513, 0.3047974513] => "
 0.32 ┼╮            ╭─╮     ╭╮     ╭
 0.29 ┤╰╮    ╭─╮   ╭╯ │    ╭╯│     │
 0.26 ┤ │   ╭╯ ╰╮ ╭╯  ╰╮  ╭╯ ╰╮   ╭╯
 0.23 ┤ ╰╮ ╭╯   ╰╮│    ╰╮╭╯   ╰╮ ╭╯
 0.20 ┤  ╰╮│     ╰╯     ╰╯     │╭╯
 0.16 ┤   ╰╯                   ╰╯    ");
 */

    graph_eq!(test_twelve ? arg.height=11 ; [
                0,0,0,0,1.5,0,0,(-0.5),9, (-3),0,0,1,2,1,0,0,0,0,
				0,0,0,0,1.5,0,0,(-0.5),8, (-3),0,0,1,2,1,0,0,0,0,
				0,0,0,0,1.5,0,0,(-0.5),10,(-3),0,0,1,2,1,0,0,0,0] => "
 10.0 ┤                                             ╭╮
  8.7 ┤       ╭╮                                    ││
  7.4 ┤       ││                 ╭╮                 ││
  6.1 ┤       ││                 ││                 ││
  4.8 ┤       ││                 ││                 ││
  3.5 ┤       ││                 ││                 ││
  2.2 ┤       ││   ╭╮            ││   ╭╮            ││   ╭╮
  0.9 ┤   ╭╮  ││  ╭╯╰╮       ╭╮  ││  ╭╯╰╮       ╭╮  ││  ╭╯╰╮
 -0.4 ┼───╯╰──╯│╭─╯  ╰───────╯╰──╯│╭─╯  ╰───────╯╰──╯│╭─╯  ╰───
 -1.7 ┤        ││                 ││                 ││
 -3.0 ┤        ╰╯                 ╰╯                 ╰╯         ");

    graph_eq!(test_thirteen ? ; [
        (-5),(-2),(-3),(-4),0,(-5),(-6),(-7),(-8),0,(-9),(-3),(-5),(-2),(-9),(-3),(-1)
    ] => "
  0.0 ┤   ╭╮   ╭╮
 -1.0 ┤   ││   ││     ╭
 -2.0 ┤╭╮ ││   ││  ╭╮ │
 -3.0 ┤│╰╮││   ││╭╮││╭╯
 -4.0 ┤│ ╰╯│   │││││││
 -5.0 ┼╯   ╰╮  │││╰╯││
 -6.0 ┤     ╰╮ │││  ││
 -7.0 ┤      ╰╮│││  ││
 -8.0 ┤       ╰╯││  ││
 -9.0 ┤         ╰╯  ╰╯ ");

    /* TODO interperse feature
    graph_eq!(test_fourteen ? arg.height=5 [
        -0.000018527,-0.021,-0.00123,0.00000021312, -0.0434321234,-0.032413241234,0.0000234234
    ] ?
        crate::Config::default().with_height(5).with_width(45) => "
  0.000 ┼─╮           ╭────────╮                    ╭
 -0.008 ┤ ╰──╮     ╭──╯        ╰─╮                ╭─╯
 -0.017 ┤    ╰─────╯             ╰╮             ╭─╯
 -0.025 ┤                         ╰─╮         ╭─╯
 -0.034 ┤                           ╰╮   ╭────╯
 -0.042 ┼                            ╰───╯           ");
 */

    graph_eq!(test_fifteen ? arg.height=25, arg.yprec=2 ; [
        57.76,54.14,56.31,57.09,59.50,52.63,53.50,56.44,56.75,52.96,55.54,55.09,58.22,56.85,60.61,
        59.62,59.73,60.15,56.30,54.69,55.32,54.03,50.98,50.48,54.55,47.49,55.30,46.74,46.00,45.80,
        49.60,48.83,47.64,46.61,54.72,42.77,50.30,42.79,41.84,44.19,43.36,45.62,45.09,44.95,50.36,
        47.21,47.77,52.04,47.46,44.19,47.22,45.55,40.65,39.64,37.26,40.71,42.15,36.45,39.14,36.62
    ] => "
 60.61 ┤             ╭╮ ╭╮
 59.60 ┤   ╭╮        │╰─╯│
 58.60 ┤   ││      ╭╮│   │
 57.59 ┼╮ ╭╯│      │││   │
 56.58 ┤│╭╯ │ ╭─╮  │╰╯   ╰╮
 55.58 ┤││  │ │ │╭─╯      │╭╮    ╭╮
 54.57 ┤╰╯  │ │ ││        ╰╯│  ╭╮││      ╭╮
 53.56 ┤    │╭╯ ││          ╰╮ ││││      ││
 52.56 ┤    ╰╯  ╰╯           │ ││││      ││
 51.55 ┤                     │ ││││      ││           ╭╮
 50.54 ┤                     ╰─╯│││      ││╭╮      ╭╮ ││
 49.54 ┤                        │││  ╭╮  ││││      ││ ││
 48.53 ┤                        │││  │╰╮ ││││      ││ ││
 47.52 ┤                        ╰╯│  │ ╰╮││││      │╰─╯╰╮╭╮
 46.52 ┤                          ╰╮ │  ╰╯│││      │    │││
 45.51 ┤                           ╰─╯    │││   ╭─╮│    ││╰╮
 44.50 ┤                                  │││ ╭╮│ ╰╯    ╰╯ │
 43.50 ┤                                  │││ │╰╯          │
 42.49 ┤                                  ╰╯╰╮│            │   ╭╮
 41.48 ┤                                     ╰╯            │   ││
 40.48 ┤                                                   ╰╮ ╭╯│
 39.47 ┤                                                    ╰╮│ │╭╮
 38.46 ┤                                                     ││ │││
 37.46 ┤                                                     ╰╯ │││
 36.45 ┤                                                        ╰╯╰ ");

}

