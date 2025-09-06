use std::fs;

use anyhow::anyhow;
use toml_edit::DocumentMut;

use crate::{
    config,
    spec::{Spec, Type},
    utils,
};

pub fn run() -> anyhow::Result<()> {
    let config =
        config::load().map_err(|e| anyhow!("Failed to load Cortex configuration: {:#?}", e))?;

    sync_service_specs(&config)
}

fn sync_service_specs(config: &config::Config) -> anyhow::Result<()> {
    println!("Checking git repository status...");

    if true || utils::is_git_repo_clean()? {
        println!("  Repository is clean.");
    } else {
        return Err(anyhow!(
            "Repository has uncommitted changes. Please commit or stash them before syncing."
        ));
    }

    println!("Syncing service specifications...");

    let services = fs::read_dir(&config.paths.specs.services)
        .map_err(|e| anyhow!("Failed to read service specifications directory: {:#?}", e))?;

    println!();
    for service in services {
        let spec_path = match service {
            Ok(entry) => entry.path(),
            Err(e) => {
                eprintln!("Failed to read service entry: {:#?}", e);
                continue;
            }
        };

        if !spec_path.is_dir() {
            continue;
        }

        println!("Processing service: {}", spec_path.display());

        let mut spec = Spec::new();

        match spec.load_dir(&spec_path) {
            Ok(()) => (),
            Err(e) => {
                eprintln!("  Failed to load specification: {:#?}", e);
                continue;
            }
        }

        println!("  Loaded specification");

        println!("  Validating specification...");

        match spec.validate() {
            Ok(()) => (),
            Err(e) => {
                eprintln!("  Validation failed: {:#?}", e);
                continue;
            }
        }

        println!("  Specification is valid.");

        println!("  Computing hash...");
        let hash = utils::dir_hash(&spec_path)?;

        println!("  Checking if service exists...");
        let service_name = spec_path
            .file_name()
            .ok_or_else(|| anyhow!("Failed to get service name"))?;

        let service_path = config.paths.services.join(service_name);
        let cargo_toml_path = service_path.join("Cargo.toml");

        if service_path.exists() {
            println!("  Service exists at {}", service_path.display());

            let cargo_config = fs::read_to_string(&cargo_toml_path)?.parse::<DocumentMut>()?;
            let metadata = &cargo_config["package"]["metadata"]["cortex"];

            let existing_hash = metadata["spec_hash"].as_str().ok_or_else(|| {
                anyhow!("No `package.metadata.cortex.spec_hash` in service Cargo.toml")
            })?;
            let spec_commit = metadata["spec_commit"].as_str().ok_or_else(|| {
                anyhow!("No `package.metadata.cortex.spec_commit` in service Cargo.toml")
            })?;

            if existing_hash == hash {
                println!("  Specification hash matches ({}). No update needed.", hash);
                continue;
            } else {
                println!(
                    "  Specification hash mismatch (existing: {}, new: {})!",
                    existing_hash, hash
                );
            }

            let last_spec_files = utils::read_committed_files_in_dir(
                spec_commit,
                spec_path
                    .to_str()
                    .ok_or_else(|| anyhow!("Specification path is not valid UTF-8"))?,
            )?;

            let mut last_spec = Spec::new();

            for file in last_spec_files {
                last_spec.load_string(&file)?;
            }

            println!(
                "  Validating previous specification... (safety measure, should always be valid)"
            );
            last_spec.validate()?;

            println!("  Previous specification is valid.");

            println!("  Computing specification changes...");

            println!();
            let changes = compute_spec_changes(last_spec, spec)?;
            println!();

            println!("  Specification changes computed: {:?}", changes);

            // TODO: bump version, write files, commit changes, set new spec hash and commit hash
            // and commit again
        } else {
            println!(
                "  Service does not exist, creating at '{}'...",
                service_path.display()
            );

            // TODO: create new service, commit, set spec and commit hash, and commit again
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum SpecChange {
    None,
    Patch,
    Minor,
    Major,
}

impl SpecChange {
    pub fn major(&mut self) {
        *self = SpecChange::Major;
    }

    pub fn minor(&mut self) {
        if *self != SpecChange::Major {
            *self = SpecChange::Minor;
        }
    }

    pub fn docs(&mut self) {
        if *self == SpecChange::None {
            *self = SpecChange::Patch;
        }
    }

    pub fn set_change(&mut self, change: SpecChange) {
        match change {
            SpecChange::Major => self.major(),
            SpecChange::Minor => self.minor(),
            SpecChange::Patch => self.docs(),
            SpecChange::None => (),
        }
    }
}

fn compute_type_changes(name: &str, old: &Type, new: &Type) -> anyhow::Result<SpecChange> {
    let mut change = SpecChange::None;

    match old {
        Type::I16
        | Type::I32
        | Type::U16
        | Type::U32
        | Type::F32
        | Type::F64
        | Type::Bool
        | Type::Timestamp
        | Type::String
        | Type::Alias(_)
        | Type::Enum(_)
        | Type::Optional(_)
        | Type::Field(_, _)
        | Type::Array(_)
        | Type::Result(_, _) => {
            if old != new {
                if old == &Type::Optional(Box::new(new.clone())) {
                    println!(
                        "  Additive change: Type '{}' changed from '{:?}' to 'Optional<{:?}>'.",
                        name, old, new
                    );
                    change.minor();
                } else {
                    println!(
                        "  Breaking change: Type '{}' changed from '{:?}' to '{:?}'.",
                        name, old, new
                    );
                    change.major();
                }
            }
        }
        Type::Struct(fields) => {
            if let Type::Struct(new_fields) = &new {
                for field in fields {
                    if let Some(new_field) = new_fields.iter().find(|f| f.name == field.name) {
                        let c = compute_type_changes(
                            &format!("{}.{}", name, field.name),
                            &field.type_,
                            &new_field.type_,
                        )?;
                        change.set_change(c);
                    } else {
                        println!(
                            "  Breaking change: Field '{}' was removed from struct '{}'.",
                            field.name, name
                        );
                        change.major();
                    }
                }

                for new_field in new_fields {
                    if !fields.iter().any(|f| f.name == new_field.name) {
                        if let Type::Optional(_) = new_field.type_ {
                            println!(
                                "  Additive change: Field '{}' was added to struct '{}' as optional.",
                                new_field.name, name
                            );
                            change.minor();
                        } else {
                            println!(
                                "  Breaking change: Field '{}' was added to struct '{}' as required.",
                                new_field.name, name
                            );
                            change.major();
                        }
                    }
                }
            } else {
                println!(
                    "  Breaking change: Type '{}' changed from 'Struct' to '{:?}'.",
                    name, new
                );
                change.major();
            }
        }
    }

    Ok(change)
}

fn compute_spec_changes(last_spec: Spec, new_spec: Spec) -> anyhow::Result<SpecChange> {
    let mut change = SpecChange::None;

    for type_spec in &last_spec.types {
        if let Some(new_type_spec) = new_spec.types.iter().find(|t| t.name == type_spec.name) {
            let c = compute_type_changes(&type_spec.name, &type_spec.kind, &new_type_spec.kind)?;
            change.set_change(c);

            if type_spec.desc != new_type_spec.desc {
                println!(
                    "  Patch change: Documentation for type '{}' was changed.",
                    type_spec.name
                );
                change.docs();
            }
        } else {
            println!("  Breaking change: Type '{}' was removed.", type_spec.name);
            change.major();
        }
    }

    for new_type_spec in &new_spec.types {
        if !last_spec.types.iter().any(|t| t.name == new_type_spec.name) {
            println!(
                "  Additive change: Type '{}' was added.",
                new_type_spec.name
            );
            change.minor();
        }
    }

    for rpc_spec in &last_spec.rpcs {
        if let Some(new_rpc_spec) = new_spec.rpcs.iter().find(|r| r.name == rpc_spec.name) {
            let c = compute_type_changes(
                &format!("RPC[{}].request", rpc_spec.name),
                &rpc_spec.request,
                &new_rpc_spec.request,
            )?;
            change.set_change(c);

            let c = compute_type_changes(
                &format!("RPC[{}].response", rpc_spec.name),
                &rpc_spec.response,
                &new_rpc_spec.response,
            )?;
            change.set_change(c);

            if rpc_spec.desc != new_rpc_spec.desc {
                println!(
                    "  Patch change: Documentation for RPC '{}' was changed.",
                    rpc_spec.name
                );
                change.docs();
            }
        } else {
            println!("  Breaking change: RPC '{}' was removed.", rpc_spec.name);
            change.major();
        }
    }

    for new_rpc_spec in &new_spec.rpcs {
        if !last_spec.rpcs.iter().any(|r| r.name == new_rpc_spec.name) {
            println!("  Additive change: RPC '{}' was added.", new_rpc_spec.name);
            change.minor();
        }
    }

    for produce_spec in &last_spec.produces {
        if let Some(new_produce_spec) = new_spec
            .produces
            .iter()
            .find(|p| p.name == produce_spec.name)
        {
            let c = compute_type_changes(
                &format!("Produce[{}].type", produce_spec.name),
                &produce_spec.type_,
                &new_produce_spec.type_,
            )?;
            change.set_change(c);

            if produce_spec.desc != new_produce_spec.desc {
                println!(
                    "  Patch change: Documentation for Produce '{}' was changed.",
                    produce_spec.name
                );
                change.docs();
            }
        } else {
            println!(
                "  Breaking change: Produce '{}' was removed.",
                produce_spec.name
            );
            change.major();
        }
    }

    for new_produce_spec in &new_spec.produces {
        if !last_spec
            .produces
            .iter()
            .any(|p| p.name == new_produce_spec.name)
        {
            println!(
                "  Additive change: Produce '{}' was added.",
                new_produce_spec.name
            );
            change.minor();
        }
    }

    for consume_spec in &last_spec.consumes {
        if let Some(new_consume_spec) = new_spec
            .consumes
            .iter()
            .find(|c| c.event == consume_spec.event && c.from == consume_spec.from)
        {
            if consume_spec.desc != new_consume_spec.desc {
                println!(
                    "  Patch change: Documentation for Consume '{}::{}' was changed.",
                    consume_spec.from, consume_spec.event
                );
                change.docs();
            }
        } else {
            println!(
                "  Minor change: Consume '{}::{}' was removed.",
                consume_spec.from, consume_spec.event
            );
            change.minor();
        }
    }

    for new_consume_spec in &last_spec.consumes {
        if !new_spec
            .consumes
            .iter()
            .any(|c| c.event == new_consume_spec.event && c.from == new_consume_spec.from)
        {
            println!(
                "  Minor change: Consume '{}::{}' was added.",
                new_consume_spec.from, new_consume_spec.event
            );
            change.minor();
        }
    }

    Ok(change)
}
