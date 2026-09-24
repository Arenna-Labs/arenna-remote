//! Arenna Remote integration glue: things that need the network or the
//! platform layer. Pure logic lives in `hbb_common::arenna`.

#[cfg(test)]
mod tests {
    #[test]
    fn is_a_first_party_client_not_a_rustdesk_custom_client() {
        assert!(!crate::common::is_custom_client());
    }

    #[test]
    fn display_name_differs_from_internal_name() {
        assert_eq!(crate::common::get_app_display_name(), "Arenna Remote");
        assert_eq!(crate::common::get_app_name(), "ArennaRemote");
    }

    #[test]
    fn translations_show_the_display_name() {
        assert_eq!(
            crate::lang::translate_locale("About RustDesk".to_owned(), "en"),
            "About Arenna Remote"
        );
        assert_eq!(
            crate::lang::translate_locale("About RustDesk".to_owned(), "es"),
            "Acerca de Arenna Remote"
        );
    }

    #[test]
    fn never_talks_to_an_api_server() {
        assert_eq!(crate::common::get_api_server(String::new(), String::new()), "");
    }
}
