use crate::{
    error::ProcessingError,
    pipeline::{FieldMask, TicketProcessor},
    ticket::{ProcessedTicket, ProcessingResult},
};
use async_trait::async_trait;
use language_enum::Language;
use log::info;
use whatlang::{Lang, detect};

pub struct LanguageProcessor;

#[async_trait]
impl TicketProcessor for LanguageProcessor {
    async fn process(&self, ticket: ProcessedTicket) -> ProcessedTicket {
        info!(
            "LanguageProcessor received event for ticket: {}",
            ticket.ticket.id
        );

        let ticket_id = ticket.ticket.id.clone();
        let lang = detect(&ticket.ticket.content).map(|info| to_language_enum(info.lang()));
        let result = ticket.with_language(lang.map_or(
            ProcessingResult::Error(ProcessingError::LanguageDetectionError()),
            ProcessingResult::Success,
        ));

        info!(
            "LanguageProcessor finished processing ticket: {}",
            ticket_id
        );
        result
    }

    fn required_fields(&self) -> FieldMask {
        FieldMask::empty()
    }

    fn output_fields(&self) -> FieldMask {
        FieldMask::LANGUAGE
    }
}

/// Maps a whatlang Lang enum to a language_enum Language enum.
/// Covers all 69 languages supported by whatlang with direct mappings
/// where possible, or uses Language::other() for languages not directly
/// represented in language_enum.
///
fn to_language_enum(lang: Lang) -> Language {
    match lang {
        // Primary world languages
        Lang::Eng => Language::English,
        Lang::Fra => Language::French,
        Lang::Spa => Language::Spanish,
        Lang::Deu => Language::German,
        Lang::Ita => Language::Italian,
        Lang::Por => Language::Portuguese,
        Lang::Rus => Language::Russian,
        Lang::Cmn => Language::Mandarin,
        Lang::Jpn => Language::Japanese,
        Lang::Kor => Language::Korean,
        Lang::Ara => Language::Arabic,
        Lang::Hin => Language::Hindi,

        // European languages
        Lang::Nld => Language::Dutch,
        Lang::Swe => Language::Swedish,
        Lang::Nob => Language::Norwegian,
        Lang::Dan => Language::Danish,
        Lang::Fin => Language::Finnish,
        Lang::Pol => Language::Polish,
        Lang::Ces => Language::Czech,
        Lang::Hun => Language::Hungarian,
        Lang::Ron => Language::Romanian,
        Lang::Bul => Language::Bulgarian,
        Lang::Hrv => Language::Croatian,
        Lang::Srp => Language::Serbian,
        Lang::Slv => Language::Slovenian,
        Lang::Slk => Language::Slovak,
        Lang::Est => Language::Estonian,
        Lang::Lav => Language::Latvian,
        Lang::Lit => Language::Lithuanian,
        Lang::Mkd => Language::Macedonian,
        Lang::Bel => Language::Belarusian,
        Lang::Ukr => Language::Ukrainian,
        Lang::Ell => Language::Greek,
        Lang::Cat => Language::Catalan,
        Lang::Tur => Language::Turkish,

        // Asian languages
        Lang::Tha => Language::Thai,
        Lang::Vie => Language::Vietnamese,
        Lang::Ind => Language::Indonesian,
        Lang::Jav => Language::Javanese,
        Lang::Tgl => Language::Tagalog,
        Lang::Mal => Language::Malayalam,
        Lang::Tam => Language::Tamil,
        Lang::Tel => Language::Telugu,
        Lang::Kan => Language::Kannada,
        Lang::Guj => Language::Gujarati,
        Lang::Ben => Language::Bengali,
        Lang::Mar => Language::Marathi,
        Lang::Ori => Language::Oriya,
        Lang::Pan => Language::Punjabi,
        Lang::Urd => Language::Urdu,
        Lang::Nep => Language::Nepali,
        Lang::Sin => Language::Sinhala,
        Lang::Mya => Language::Burmese,
        Lang::Khm => Language::Khmer,

        // Middle Eastern and Central Asian languages
        Lang::Pes => Language::Farsi,
        Lang::Heb => Language::Hebrew,
        Lang::Aze => Language::Azeri,
        Lang::Uzb => Language::Uzbek,
        Lang::Tuk => Language::Turkmen,

        // Caucasian languages
        Lang::Kat => Language::Georgian,
        Lang::Hye => Language::Armenian,

        // African languages
        Lang::Amh => Language::Amharic,
        Lang::Afr => Language::Afrikaans,
        Lang::Zul => Language::Zulu,
        Lang::Sna => Language::Shona,
        Lang::Aka => Language::other("Akan".to_string()),

        // European minority/regional languages
        Lang::Yid => Language::other("Yiddish".to_string()),
        Lang::Lat => Language::Latin,
        Lang::Epo => Language::other("Esperanto".to_string()),
    }
}
