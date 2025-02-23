CREATE SCHEMA aesterisk;

CREATE TABLE aesterisk.roles (
	role_id SERIAL PRIMARY KEY NOT NULL,
	role_name TEXT NOT NULL,
	role_parent INTEGER DEFAULT NULL,
	CONSTRAINT fk_roles FOREIGN KEY(role_parent) REFERENCES aesterisk.roles(role_id)
);

CREATE TABLE aesterisk.permissions (
	permission_id SERIAL PRIMARY KEY NOT NULL,
	permission_name TEXT NOT NULL
);

CREATE TABLE aesterisk.role_permissions (
	role_id INTEGER NOT NULL,
	permission_id INTEGER NOT NULL,
	CONSTRAINT fk_roles FOREIGN KEY(role_id) REFERENCES aesterisk.roles(role_id),
	CONSTRAINT fk_permissions FOREIGN KEY(permission_id) REFERENCES aesterisk.permissions(permission_id),
	PRIMARY KEY(role_id, permission_id)
);

CREATE INDEX ix_role_permissions_permission ON aesterisk.role_permissions(permission_id);

CREATE TABLE aesterisk.nodes (
	node_id SERIAL PRIMARY KEY NOT NULL,
	node_name TEXT NOT NULL,
	node_last_active_at TIMESTAMP DEFAULT NULL,
	node_public_key TEXT NOT NULL,
	node_last_external_ip VARCHAR(15) DEFAULT NULL,
	node_ip_locked BOOLEAN NOT NULL,
	node_uuid UUID NOT NULL
);

CREATE INDEX ix_nodes_uuid ON aesterisk.nodes(node_uuid);

CREATE TABLE aesterisk.networks (
	network_id SERIAL PRIMARY KEY NOT NULL,
	network_name TEXT NOT NULL,
	-- network_docker_id TEXT DEFAULT NULL,
	network_local_ip SMALLINT NOT NULL
);

CREATE TABLE aesterisk.node_networks (
	node_id INTEGER NOT NULL,
	network_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_nodes FOREIGN KEY(node_id) REFERENCES aesterisk.nodes(node_id),
	CONSTRAINT fk_networks FOREIGN KEY(network_id) REFERENCES aesterisk.networks(network_id),
	PRIMARY KEY(node_id, network_id)
);

CREATE INDEX ix_node_networks_network ON aesterisk.node_networks(network_id);

CREATE TABLE aesterisk.templates (
	template_id SERIAL PRIMARY KEY NOT NULL,
	template_name TEXT NOT NULL,
	template_author TEXT DEFAULT NULL,
	template_team INTEGER DEFAULT NULL,
	template_description TEXT NOT NULL,
	CONSTRAINT fk_teams FOREIGN KEY(template_team) REFERENCES aesterisk.teams(team_id)
);

CREATE TABLE aesterisk.tags (
	tag_id SERIAL PRIMARY KEY NOT NULL,
	tag_name TEXT NOT NULL,
	tag_image TEXT NOT NULL,
	tag_docker_tags TEXT NOT NULL,
	tag_healthcheck_test TEXT[] NOT NULL,
	tag_healthcheck_interval INTEGER NOT NULL,
	tag_healthcheck_timeout INTEGER NOT NULL,
	tag_healthcheck_retries INTEGER NOT NULL
);

CREATE TABLE aesterisk.template_tags (
	template_id INTEGER NOT NULL,
	tag_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_templates FOREIGN KEY(template_id) REFERENCES aesterisk.templates(template_id),
	CONSTRAINT fk_tags FOREIGN KEY(tag_id) REFERENCES aesterisk.tags(tag_id),
	PRIMARY KEY(template_id, tag_id)
);

CREATE INDEX ix_template_tags_tag ON aesterisk.template_tags(tag_id);

CREATE TABLE aesterisk.env_defs (
	env_def_id SERIAL PRIMARY KEY NOT NULL,
	env_def_name TEXT NOT NULL,
	env_def_description TEXT NOT NULL,
	env_def_key TEXT NOT NULL,
	env_def_secret BOOLEAN NOT NULL,
	env_def_required BOOLEAN NOT NULL,
	env_def_type SMALLINT NOT NULL,
	env_def_default_value TEXT DEFAULT NULL,
	env_def_regex TEXT DEFAULT NULL,
	env_def_min INTEGER DEFAULT NULL,
	env_def_max INTEGER DEFAULT NULL,
	env_def_trim BOOLEAN NOT NULL
);

CREATE TABLE aesterisk.tag_env_defs (
	tag_id INTEGER NOT NULL,
	env_def_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_tags FOREIGN KEY(tag_id) REFERENCES aesterisk.tags(tag_id),
	CONSTRAINT fk_env_defs FOREIGN KEY(env_def_id) REFERENCES aesterisk.env_defs(env_def_id),
	PRIMARY KEY(tag_id, env_def_id)
);

CREATE INDEX ix_tag_env_defs_env_def ON aesterisk.tag_env_defs(env_def_id);

CREATE TABLE aesterisk.port_defs (
	port_def_id SERIAL PRIMARY KEY NOT NULL,
	port_def_name TEXT NOT NULL,
	port_def_description TEXT NOT NULL,
	port_def_port INTEGER NOT NULL,
	port_def_protocol SMALLINT NOT NULL
);

CREATE TABLE aesterisk.tag_port_defs (
	tag_id INTEGER NOT NULL,
	port_def_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_tags FOREIGN KEY(tag_id) REFERENCES aesterisk.tags(tag_id),
	CONSTRAINT fk_port_defs FOREIGN KEY(port_def_id) REFERENCES aesterisk.port_defs(port_def_id),
	PRIMARY KEY(tag_id, port_def_id)
);

CREATE INDEX ix_tag_port_defs_port_def ON aesterisk.tag_port_defs(port_def_id);

CREATE TABLE aesterisk.servers (
	server_id SERIAL PRIMARY KEY NOT NULL,
	server_name TEXT NOT NULL,
	-- server_docker_id TEXT DEFAULT NULL,
	server_tag INTEGER NOT NULL,
	CONSTRAINT fk_tags FOREIGN KEY(server_tag) REFERENCES aesterisk.tags(tag_id)
);

CREATE TABLE aesterisk.ports (
	port_id SERIAL PRIMARY KEY NOT NULL,
	port_port INTEGER NOT NULL,
	port_protocol SMALLINT NOT NULL,
	port_mapped INTEGER NOT NULL
);

CREATE TABLE aesterisk.server_ports (
	server_id INTEGER NOT NULL,
	port_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_servers FOREIGN KEY(server_id) REFERENCES aesterisk.servers(server_id),
	CONSTRAINT fk_ports FOREIGN KEY(port_id) REFERENCES aesterisk.ports(port_id),
	PRIMARY KEY(server_id, port_id)
);

CREATE TABLE aesterisk.envs (
	env_id SERIAL PRIMARY KEY NOT NULL,
	env_key TEXT NOT NULL,
	env_value TEXT NOT NULL,
	env_secret BOOLEAN NOT NULL,
);

CREATE TABLE aesterisk.server_envs (
	server_id INTEGER NOT NULL,
	env_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_servers FOREIGN KEY(server_id) REFERENCES aesterisk.servers(server_id),
	CONSTRAINT fk_envs FOREIGN KEY(env_id) REFERENCES aesterisk.envs(env_id),
	PRIMARY KEY(server_id, env_id)
);

CREATE TABLE aesterisk.server_networks (
	server_id INTEGER NOT NULL,
	network_id INTEGER NOT NULL,
	local_ip SMALLINT NOT NULL,
	CONSTRAINT fk_servers FOREIGN KEY(server_id) REFERENCES aesterisk.servers(server_id),
	CONSTRAINT fk_networks FOREIGN KEY(network_id) REFERENCES aesterisk.networks(network_id),
	PRIMARY KEY(server_id, network_id)
);

CREATE INDEX ix_server_networks_network ON aesterisk.server_networks(network_id);

CREATE TABLE aesterisk.node_servers (
	node_id INTEGER NOT NULL,
	server_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_nodes FOREIGN KEY(node_id) REFERENCES aesterisk.nodes(node_id),
	CONSTRAINT fk_servers FOREIGN KEY(server_id) REFERENCES aesterisk.servers(server_id),
	PRIMARY KEY(node_id, server_id)
);

CREATE INDEX ix_node_servers_server ON aesterisk.node_servers(server_id);

CREATE TABLE aesterisk.teams (
	team_id SERIAL PRIMARY KEY NOT NULL,
	team_path VARCHAR(64),
	team_name TEXT NOT NULL,
	team_plan SMALLINT NOT NULL,
	team_is_personal BOOLEAN NOT NULL,
	team_created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE aesterisk.team_nodes (
	team_id INTEGER NOT NULL,
	node_id INTEGER NOT NULL UNIQUE,
	CONSTRAINT fk_teams FOREIGN KEY(team_id) REFERENCES aesterisk.teams(team_id),
	CONSTRAINT fk_nodes FOREIGN KEY(node_id) REFERENCES aesterisk.nodes(node_id),
	PRIMARY KEY(team_id, node_id)
);

CREATE INDEX ix_team_nodes_node ON aesterisk.team_nodes(node_id);

CREATE TABLE aesterisk.accounts (
	account_id SERIAL PRIMARY KEY NOT NULL,
	account_gh_id TEXT NOT NULL,
	account_email TEXT NOT NULL,
	account_first_name TEXT NOT NULL,
	account_last_name TEXT DEFAULT NULL,
	account_avatar TEXT DEFAULT NULL,
	account_created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	account_last_active_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	account_personal_team INTEGER NOT NULL,
	CONSTRAINT fk_teams FOREIGN KEY(account_personal_team) REFERENCES aesterisk.teams(team_id)
);

CREATE INDEX ix_accounts_gh_id ON aesterisk.accounts(account_gh_id);

CREATE TABLE aesterisk.users (
	user_id SERIAL PRIMARY KEY NOT NULL,
	user_account INTEGER NOT NULL,
	user_team INTEGER NOT NULL,
	user_joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	user_owner BOOLEAN NOT NULL,
	user_public_key TEXT NOT NULL,
	user_private_key TEXT NOT NULL,
	CONSTRAINT fk_accounts FOREIGN KEY(user_account) REFERENCES aesterisk.accounts(account_id),
	CONSTRAINT fk_teams FOREIGN KEY(user_team) REFERENCES aesterisk.teams(team_id)
);

CREATE INDEX ix_users_account ON aesterisk.users(user_account);
CREATE INDEX ix_users_team ON aesterisk.users(user_team);

CREATE TABLE aesterisk.user_roles (
	user_id INTEGER NOT NULL,
	role_id INTEGER NOT NULL,
	CONSTRAINT fk_users FOREIGN KEY(user_id) REFERENCES aesterisk.users(user_id),
	CONSTRAINT fk_roles FOREIGN KEY(role_id) REFERENCES aesterisk.roles(role_id),
	PRIMARY KEY(user_id, role_id)
);

CREATE INDEX ix_user_roles_role ON aesterisk.user_roles(role_id);
