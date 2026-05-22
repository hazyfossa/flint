pub mod dbus {
    // Varlink login api does not support seat events
    pub mod logind;
}

pub mod varlink {
    #![allow(dead_code)]
    pub mod userdb;
}
