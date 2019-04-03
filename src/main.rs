use std::{path::PathBuf, time::Duration};

use failure::{err_msg, format_err, Error};
use lazy_static::lazy_static;
use structopt::StructOpt;

include!(concat!(env!("OUT_DIR"), "/lang.codegen.rs"));

lazy_static! {
    static ref AUTODETECT: &'static str = LANGUAGES.get_key("autodetect").unwrap();
    static ref ONE_MINUTE: Duration = Duration::from_secs(60);
    static ref ONE_HOUR: Duration = ONE_MINUTE.checked_mul(60).unwrap();
    static ref ONE_DAY: Duration = ONE_HOUR.checked_mul(24).unwrap();
    static ref ONE_WEEK: Duration = ONE_DAY.checked_mul(7).unwrap();
    static ref ONE_MONTH: Duration = ONE_WEEK.checked_mul(4).unwrap();
    static ref ONE_YEAR: Duration = ONE_DAY.checked_mul(365).unwrap();
    static ref ONE_HUNDRED_YEARS: Duration = ONE_YEAR.checked_mul(100).unwrap();
}

#[derive(Debug, StructOpt)]
struct Options {
    /// Your pastery API key.
    ///
    /// You can find this at https://www.pastery.net/account/.
    #[structopt(long = "api-key", env = "PASTERY_API_KEY")]
    api_key: String,

    /// The alias of the programming language that the paste is written in.
    ///
    /// If not provided, Pastery will auto-detect the language.
    #[structopt(
        long = "lang",
        default_value = "autodetect",
        parse(from_str = "parse_lang")
    )]
    lang: &'static str,

    /// The duration that this paste will live for.
    ///
    /// After this time, the paste will be deleted. The default duration is one day.
    #[structopt(
        long = "duration",
        default_value = "1d",
        parse(try_from_str = "parse_duration")
    )]
    duration: Duration,

    /// The title of the paste.
    #[structopt(long = "title")]
    title: Option<String>,

    /// The number of views after which this paste will expire.
    ///
    /// If not provided, the paste will not have view-based expiration.
    #[structopt(long = "max-views", parse(try_from_str))]
    max_views: Option<u32>,

    /// The path of the file to upload.
    ///
    /// If not provided, the file will be read from standard input.
    path: Option<PathBuf>,
}

fn parse_lang(lang: &str) -> &'static str {
    LANGUAGES
        .get_key(lang)
        .cloned()
        .unwrap_or_else(|| *AUTODETECT)
}

fn parse_duration(s: &str) -> Result<Duration, Error> {
    if let Some(split_at) = s.find(|c: char| !c.is_ascii_digit()) {
        let (amount_s, unit) = s.split_at(split_at);
        let amount = amount_s.parse::<u32>()?;

        let unit = match unit {
            "m" => *ONE_MINUTE,
            "h" => *ONE_HOUR,
            "d" => *ONE_DAY,
            "w" => *ONE_WEEK,
            "mo" => *ONE_MONTH,
            "y" => *ONE_YEAR,
            _ => {
                return Err(format_err!(
                    "Unknown unit {}, expected one of m, h, d, w, mo, y",
                    unit
                ));
            }
        };

        match unit.checked_mul(amount) {
            Some(duration) => {
                if duration > *ONE_HUNDRED_YEARS {
                    Err(format_err!(
                        "Duration {} is too long; maximum duration is 100y",
                        s
                    ))
                } else {
                    Ok(duration)
                }
            }

            None => Err(format_err!(
                "Duration {} is too long; maximum duration is 100y",
                s
            )),
        }
    } else {
        Err(err_msg(
            "Did not find a unit, expected one of m, h, d, w, y",
        ))
    }
}

fn main() {
    let options = Options::from_args();
    println!("{:?}", options);
}
