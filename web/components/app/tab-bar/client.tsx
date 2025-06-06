"use client";

import { isPathnameActive } from "@/lib/utils";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useMemo, useState } from "react";
import { motion } from "motion/react";

export type Tab = {
	name: string;
	path: string;
};

export function Tabs({ tabs, level = 2 }: Readonly<{
	tabs: Tab[];
	level?: number;
}>) {
	const pathname = usePathname();

	const baseUrl = useMemo(() => pathname.split("/").slice(0, level + 1).join("/"), [pathname, level]);

	const linkedTabs = useMemo(() => tabs.map((tab) => ({
		name: tab.name,
		path: `${baseUrl}/${tab.path.charAt(0) === "/" ? tab.path.slice(1) : tab.path}`,
	} satisfies Tab)), [baseUrl, tabs]);

	const finalTabs = useMemo(() => linkedTabs.map((tab) => ({
		name: tab.name,
		path: tab.path,
		active: isPathnameActive(tab.path, pathname),
	} satisfies Tab & { active: boolean; })), [linkedTabs, pathname]);

	const [hoveredTab, setHoveredTab] = useState<number | null>(null);

	return (
		<div className="flex flex-row gap-0 items-center h-full px-2">
			{
				finalTabs.map((tab, index) => (
					<Link
						href={tab.path}
						className="h-12 items-center flex flex-row relative"
						key={`tab-${tab.path}`}
						onMouseEnter={() => setHoveredTab(index)}
						onMouseLeave={() => setHoveredTab(null)}
					>
						<button className="relative cursor-pointer bg-none h-9 px-4 py-2 inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive">
							{
								hoveredTab === index && (
									<motion.div
										layoutId="tab-hover"
										className="-inset-x-2 h-full rounded-md absolute bottom-0 -left-2 bg-accent dark:bg-accent/50 pointer-events-none"
										transition={
											{
												opacity: {
													ease: "easeOut",
													duration: 2.2,
												},
												layout: {
													type: "spring",
													damping: 40,
													stiffness: 400,
												},
											}
										}
									/>
								)
							}
							<span className="text-accent-foreground z-20 font-normal">{ tab.name }</span>
						</button>
						{
							tab.active && (
								<motion.div
									layoutId="tab-underline"
									className="absolute bottom-0 left-0 h-0.5 w-full bg-accent-foreground rounded-md pointer-events-none"
									transition={
										{
											type: "spring",
											damping: 40,
											stiffness: 400,
										}
									}
								/>
							)
						}
					</Link>
				))
			}
		</div>
	);
}
