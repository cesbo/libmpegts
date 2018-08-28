
#[derive(Debug)]
pub struct Lang {
    iso639_1: &'static str,
    iso639_2t: &'static str,
    iso639_2b: &'static str,
    name: &'static str,
}

macro_rules! lang {
    ($(($x1:expr, $x2:expr, $x3:expr, $x4:expr)),*) => {{ [
        $(
            Lang { iso639_1: $x1, iso639_2t: $x2, iso639_2b: $x3, name: $x4 },
        )*
    ] }};
}

pub const LANG_LIST: &'static [Lang] = &lang![
    ("ab", "abk", "abk", "Abkhaz"),
    ("af", "afr", "afr", "Afrikaans"),
    ("ak", "aka", "aka", "Akan"),
    ("sq", "sqi", "alb", "Albanian"),
    ("am", "amh", "amh", "Amharic"),
    ("ar", "ara", "ara", "Arabic"),
    ("hy", "hye", "arm", "Armenian"),
    ("as", "asm", "asm", "Assamese"),
    ("ay", "aym", "aym", "Aymara"),
    ("az", "aze", "aze", "Azerbaijani"),
    ("bm", "bam", "bam", "Bambara"),
    ("ba", "bak", "bak", "Bashkir"),
    ("eu", "eus", "baq", "Basque"),
    ("be", "bel", "bel", "Belarusian"),
    ("bn", "ben", "ben", "Bengali"),
    ("bs", "bos", "bos", "Bosnian"),
    ("bg", "bul", "bul", "Bulgarian"),
    ("my", "mya", "bur", "Burmese"),
    ("ca", "cat", "cat", "Catalan"),
    ("ny", "nya", "nya", "Chichewa"),
    ("zh", "zho", "chi", "Chinese"),
    ("cv", "chv", "chv", "Chuvash"),
    ("co", "cos", "cos", "Corsican"),
    ("hr", "hrv", "hrv", "Croatian"),
    ("cs", "ces", "cze", "Czech"),
    ("da", "dan", "dan", "Danish"),
    ("dv", "div", "div", "Dhivehi"),
    ("nl", "nld", "dut", "Dutch"),
    ("dz", "dzo", "dzo", "Dzongkha"),
    ("en", "eng", "eng", "English"),
    ("et", "est", "est", "Estonian"),
    ("ee", "ewe", "ewe", "Ewe"),
    ("fj", "fij", "fij", "Fijian"),
    ("fi", "fin", "fin", "Finnish"),
    ("fr", "fra", "fre", "French"),
    ("ff", "ful", "ful", "Fulah"),
    ("gl", "glg", "glg", "Galician"),
    ("ka", "kat", "geo", "Georgian"),
    ("de", "deu", "ger", "German"),
    ("el", "ell", "gre", "Greek"),
    ("gn", "grn", "grn", "Guarani"),
    ("gu", "guj", "guj", "Gujarati"),
    ("ht", "hat", "hat", "Haitian"),
    ("ha", "hau", "hau", "Hausa"),
    ("he", "heb", "heb", "Hebrew"),
    ("hi", "hin", "hin", "Hindi"),
    ("hu", "hun", "hun", "Hungarian"),
    ("id", "ind", "ind", "Indonesian"),
    ("ga", "gle", "gle", "Irish"),
    ("ig", "ibo", "ibo", "Igbo"),
    ("is", "isl", "ice", "Icelandic"),
    ("it", "ita", "ita", "Italian"),
    ("ja", "jpn", "jpn", "Japanese"),
    ("kl", "kal", "kal", "Greenlandic"),
    ("kn", "kan", "kan", "Kannada"),
    ("kr", "kau", "kau", "Kanuri"),
    ("ks", "kas", "kas", "Kashmiri"),
    ("kk", "kaz", "kaz", "Kazakh"),
    ("km", "khm", "khm", "Khmer"),
    ("ki", "kik", "kik", "Kikuyu"),
    ("rw", "kin", "kin", "Kinyarwanda"),
    ("ky", "kir", "kir", "Kyrgyz"),
    ("kg", "kon", "kon", "Kongo"),
    ("ko", "kor", "kor", "Korean"),
    ("ku", "kur", "kur", "Kurdish"),
    ("lb", "ltz", "ltz", "Luxembourgish"),
    ("lg", "lug", "lug", "Ganda"),
    ("li", "lim", "lim", "Limburgish"),
    ("ln", "lin", "lin", "Lingala"),
    ("lo", "lao", "lao", "Lao"),
    ("lt", "lit", "lit", "Lithuanian"),
    ("lv", "lav", "lav", "Latvian"),
    ("mk", "mkd", "mac", "Macedonian"),
    ("mg", "mlg", "mlg", "Malagasy"),
    ("ms", "msa", "may", "Malay"),
    ("ml", "mal", "mal", "Malayalam"),
    ("mt", "mlt", "mlt", "Maltese"),
    ("mr", "mar", "mar", "Marathi"),
    ("mh", "mah", "mah", "Marshallese"),
    ("mn", "mon", "mon", "Mongolian"),
    ("na", "nau", "nau", "Nauruan"),
    ("nd", "nde", "nde", "Northern Ndebele"),
    ("ne", "nep", "nep", "Nepali"),
    ("no", "nor", "nor", "Norwegian"),
    ("nr", "nbl", "nbl", "Southern Ndebele"),
    ("oc", "oci", "oci", "Occitan"),
    ("om", "orm", "orm", "Oromo"),
    ("or", "ori", "ori", "Oriya"),
    ("os", "oss", "oss", "Ossetian"),
    ("pa", "pan", "pan", "Punjabi"),
    ("fa", "fas", "per", "Persian (Farsi)"),
    ("pl", "pol", "pol", "Polish"),
    ("ps", "pus", "pus", "Pashto"),
    ("pt", "por", "por", "Portuguese"),
    ("qu", "que", "que", "Quechua"),
    ("rn", "run", "run", "Kirundi"),
    ("ro", "ron", "rum", "Romanian"),
    ("ru", "rus", "rus", "Russian"),
    ("sa", "san", "san", "Sanskrit"),
    ("sc", "srd", "srd", "Sardinian"),
    ("sd", "snd", "snd", "Sindhi"),
    ("sm", "smo", "smo", "Samoan"),
    ("sg", "sag", "sag", "Sango"),
    ("sr", "srp", "srp", "Serbian"),
    ("gd", "gla", "gla", "Gaelic"),
    ("sn", "sna", "sna", "Shona"),
    ("si", "sin", "sin", "Sinhalese"),
    ("sk", "slk", "slo", "Slovak"),
    ("sl", "slv", "slv", "Slovene"),
    ("so", "som", "som", "Somali"),
    ("st", "sot", "sot", "Sesotho"),
    ("es", "spa", "spa", "Spanish"),
    ("su", "sun", "sun", "Sundanese"),
    ("sw", "swa", "swa", "Swahili"),
    ("ss", "ssw", "ssw", "Swati"),
    ("sv", "swe", "swe", "Swedish"),
    ("ta", "tam", "tam", "Tamil"),
    ("te", "tel", "tel", "Telugu"),
    ("tg", "tgk", "tgk", "Tajik"),
    ("th", "tha", "tha", "Thai"),
    ("ti", "tir", "tir", "Tigrinya"),
    ("bo", "bod", "tib", "Tibetan"),
    ("tk", "tuk", "tuk", "Turkmen"),
    ("tl", "tgl", "tgl", "Tagalog"),
    ("tn", "tsn", "tsn", "Tswana"),
    ("to", "ton", "ton", "Tonga"),
    ("tr", "tur", "tur", "Turkish"),
    ("ts", "tso", "tso", "Tsonga"),
    ("tt", "tat", "tat", "Tatar"),
    ("tw", "twi", "twi", "Twi"),
    ("ty", "tah", "tah", "Tahitian"),
    ("ug", "uig", "uig", "Uyghur"),
    ("uk", "ukr", "ukr", "Ukrainian"),
    ("ur", "urd", "urd", "Urdu"),
    ("uz", "uzb", "uzb", "Uzbek"),
    ("ve", "ven", "ven", "Venda"),
    ("vi", "vie", "vie", "Vietnamese"),
    ("wa", "wln", "wln", "Walloon"),
    ("cy", "cym", "wel", "Welsh"),
    ("wo", "wol", "wol", "Wolof"),
    ("fy", "fry", "fry", "Frisian"),
    ("xh", "xho", "xho", "Xhosa"),
    ("yi", "yid", "yid", "Yiddish"),
    ("yo", "yor", "yor", "Yoruba"),
    ("za", "zha", "zha", "Zhuang"),
    ("zu", "zul", "zul", "Zulu")
];
