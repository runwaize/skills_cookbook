//! Unit tests for CLI: init, doctor, config integration.

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn init_creates_chef_and_guest_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());

        let chef = temp.path().join("chef");
        let guest = temp.path().join("guest");

        let args = crate::init::InitArgs {
            chef_dir: Some(chef.clone()),
            guest_dir: Some(guest.clone()),
        };
        crate::init::run_init(&args).unwrap();

        assert!(chef.exists(), "chef dir should exist");
        assert!(chef.is_dir());
        assert!(guest.exists(), "guest dir should exist");
        assert!(guest.is_dir());
        assert!(chef.join(".git").exists(), "chef should be a git repo");

        env::remove_var("CONFIG_DIR");
    }

    #[test]
    #[serial]
    fn doctor_offline_passes_when_dirs_exist() {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("cfg");
        std::fs::create_dir_all(&config_dir).unwrap();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());

        let relay_dir = config_dir.join("skill-cookbook-relay");
        std::fs::create_dir_all(&relay_dir).unwrap();
        let chef = temp.path().join("chef");
        let guest = temp.path().join("guest");
        std::fs::create_dir_all(&chef).unwrap();
        std::fs::create_dir_all(&guest).unwrap();
        let _ = std::process::Command::new("git").args(["init"]).current_dir(&chef).status();
        let mut config = skill_cookbook_relay::config::Config::default();
        config.chef_dir = chef.clone();
        config.guest_dir = guest.clone();
        config.save().unwrap();

        let args = crate::doctor::DoctorArgs { offline: true };
        let result = crate::doctor::run_doctor(&args);

        env::remove_var("CONFIG_DIR");
        result.expect("doctor --offline should pass when dirs and config exist");
    }

    #[test]
    #[serial]
    fn doctor_offline_fails_when_chef_dir_missing() {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("cfg");
        std::fs::create_dir_all(&config_dir).unwrap();
        env::set_var("CONFIG_DIR", config_dir.as_os_str());

        let relay_dir = config_dir.join("skill-cookbook-relay");
        std::fs::create_dir_all(&relay_dir).unwrap();
        let chef = temp.path().join("nonexistent_chef");
        let guest = temp.path().join("guest");
        std::fs::create_dir_all(&guest).unwrap();
        let mut config = skill_cookbook_relay::config::Config::default();
        config.chef_dir = chef;
        config.guest_dir = guest;
        config.save().unwrap();

        let args = crate::doctor::DoctorArgs { offline: true };
        let result = crate::doctor::run_doctor(&args);

        env::remove_var("CONFIG_DIR");
        assert!(result.is_err());
    }
}
