use std::{path::PathBuf, time::Duration};

use failure::{err_msg, format_err, Error};
use lazy_static::lazy_static;
use structopt::StructOpt;
use url::Url;

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
    static ref PASTERY_URL: &'static str = "https://www.pastery.net/api/paste/";
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
    ///
    /// If not provided, the name of the file will be used instead.
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

fn generate_url(options: &Options) -> Url {
    let mut url = Url::parse(*PASTERY_URL).unwrap();
    {
        let mut query_pairs = url.query_pairs_mut();

        let duration_in_min = options.duration.as_secs() / 60;

        query_pairs
            .append_pair("api_key", &options.api_key)
            .append_pair("language", options.lang)
            .append_pair("duration", &duration_in_min.to_string());

        let max_views = options.max_views.unwrap_or(0);
        if max_views > 0 {
            query_pairs.append_pair("max_views", &max_views.to_string());
        }

        let maybe_title = match (&options.title, &options.path) {
            (Some(ref title), _) => Some(title.clone()),
            (_, Some(ref path)) => path
                .file_name()
                .map(std::ffi::OsStr::to_string_lossy)
                .map(String::from),
            (_, _) => None,
        };

        if let Some(title) = maybe_title {
            query_pairs.append_pair("title", &title);
        }
    }

    url
}

fn main() {
    let options = Options::from_args();
    println!("{:?}", options);

    let url = generate_url(&options);
    println!("url = {:?}", url);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_langs() {
        assert_eq!(&parse_lang(*AUTODETECT), &*AUTODETECT);
        assert_eq!(&parse_lang("rust"), LANGUAGES.get_key("rust").unwrap());
        assert_eq!(&parse_lang("c"), LANGUAGES.get_key("c").unwrap());
        assert_eq!(&parse_lang("html"), LANGUAGES.get_key("html").unwrap());
        assert_eq!(&parse_lang("python"), LANGUAGES.get_key("python").unwrap());
        assert_eq!(&parse_lang(""), &*AUTODETECT);
        assert_eq!(&parse_lang("asdf"), &*AUTODETECT);
    }

    #[test]
    fn parse_durations() {
        assert_eq!(parse_duration("1m").unwrap(), *ONE_MINUTE);
        assert_eq!(
            parse_duration("5m").unwrap(),
            ONE_MINUTE.checked_mul(5).unwrap()
        );
        assert_eq!(parse_duration("1d").unwrap(), *ONE_DAY);
        assert_eq!(parse_duration("1w").unwrap(), *ONE_WEEK);
        assert_eq!(parse_duration("1mo").unwrap(), *ONE_MONTH);
        assert_eq!(parse_duration("1y").unwrap(), *ONE_YEAR);
        assert_eq!(parse_duration("100y").unwrap(), *ONE_HUNDRED_YEARS);

        assert!(parse_duration("101y").is_err());
        assert!(parse_duration("m").is_err());
        assert!(parse_duration("100").is_err());
        assert!(parse_duration("100j").is_err());
    }

    #[test]
    fn generate_urls() {
        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_MINUTE,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_HOUR,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=60"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_WEEK,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=10080"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_MONTH,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=40320"
        );
        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_YEAR,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=525600"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_HUNDRED_YEARS,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=52560000"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: LANGUAGES.get_key("rust").unwrap(),
                duration: *ONE_DAY,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=rust&duration=1440"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: LANGUAGES.get_key("c").unwrap(),
                duration: *ONE_DAY,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=c&duration=1440"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "bar".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: None,
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=bar&language=autodetect&duration=1440"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: Some(0),
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: Some(100),
                title: None,
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440&max_views=100"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: None,
                title: Some("foo bar.rs".into()),
                path: None,
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440&title=foo+bar.rs"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: None,
                title: Some("foo bar.rs".into()),
                path: Some(PathBuf::from("foo.rs")),
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440&title=foo+bar.rs"
        );

        assert_eq!(
            generate_url(&Options {
                api_key: "foo".into(),
                lang: *AUTODETECT,
                duration: *ONE_DAY,
                max_views: None,
                title: None,
                path: Some(PathBuf::from("foo").join("bar.rs")),
            })
            .to_string(),
            "https://www.pastery.net/api/paste/?api_key=foo&language=autodetect&duration=1440&title=bar.rs"
        );
    }
}
