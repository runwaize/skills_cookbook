//! Library and skill management (chef). Stubs until backend APIs are available.

#[derive(clap::Args, Debug)]
pub struct LibraryAddArgs {
    pub name: String,
}

#[derive(clap::Args, Debug)]
pub struct LibraryRemoveArgs {
    pub id: String,
}

#[derive(clap::Args, Debug)]
pub struct LibraryAddSkillArgs {
    pub lib_id: String,
    pub skill_id: String,
}

#[derive(clap::Args, Debug)]
pub struct LibraryRemoveSkillArgs {
    pub lib_id: String,
    pub skill_id: String,
}

#[derive(clap::Args, Debug)]
pub struct SkillStatusArgs {
    pub skill_id: String,
    pub status: String,
}

pub fn run_library_add(
    _args: &LibraryAddArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("library add: API not yet available (backend endpoint TBD)");
    Ok(())
}

pub fn run_library_remove(
    _args: &LibraryRemoveArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("library remove: API not yet available (backend endpoint TBD)");
    Ok(())
}

pub fn run_library_add_skill(
    _args: &LibraryAddSkillArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("library add-skill: API not yet available (backend endpoint TBD)");
    Ok(())
}

pub fn run_library_remove_skill(
    _args: &LibraryRemoveSkillArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("library remove-skill: API not yet available (backend endpoint TBD)");
    Ok(())
}

pub fn run_skill_status(
    _args: &SkillStatusArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("skill status: API not yet available (backend endpoint TBD)");
    Ok(())
}
