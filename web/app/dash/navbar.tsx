import { Tab, Tabs } from "@app/tab-bar/client";
import { TabAccountDropdown, TabAccountDropdownLabel, TabAccountDropdownLink, TabAccountDropdownLogout, TabAccountDropdownSeparator, TabBar, TabLogo, TabSpacer } from "@app/tab-bar/server";
import { TeamSwitcher } from "@app/team-switcher/server";
import { User } from "lucide-react";

export default function Navbar({ tabs, level = 2, team }: Readonly<{
	tabs: Tab[];
	level?: number;
	team: Promise<string>;
}>) {
	return (
		<TabBar>
			<TabLogo />
			<Tabs tabs={tabs} level={level} />
			<TabSpacer />
			<TeamSwitcher team={team} className="mr-2" />
			<TabAccountDropdown>
				<TabAccountDropdownLabel />
				<TabAccountDropdownSeparator />
				<TabAccountDropdownLink href="/account" label="Manage Account" icon={<User className="size-4 text-foreground/50" />} />
				<TabAccountDropdownLogout />
			</TabAccountDropdown>
		</TabBar>
	);
}
