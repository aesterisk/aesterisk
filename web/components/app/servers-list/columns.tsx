"use client";

import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { ServerData } from ".";
import { Skeleton } from "@/components/ui/skeleton";
import { cn, mapValue } from "@/lib/utils";
import { Progress } from "@/components/ui/progress";

export const columns: ColumnDef<ServerData>[] = [
	{
		id: "name",
		header: () => <span className="w-min h-min select">{ "Server" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/servers/[serverID]
			<Link href="" className="select">{ row.original.name }</Link>
		),
	},
	{
		id: "status",
		header: () => <span className="w-min h-min select">{ "Status" }</span>,
		cell: ({ row }) => {
			// eslint-disable-next-line no-undefined
			if(row.original.status === undefined) {
				return (
					<Skeleton className="h-2 w-20" />
				);
			}

			return (
				<div className="flex items-center gap-2">
					<div
						className={
							cn(
								"rounded-full bg-rose-600 w-2 h-2",
								row.original.status === "healthy"
									? "bg-emerald-500"
									: (
										(row.original.status === "starting" || row.original.status === "restarting" || row.original.status === "stopping")
											? "bg-yellow-500"
											: (
												row.original.status === "unhealthy"
													? "bg-rose-600"
													: "bg-primary/20"
											)
									),
							)
						}
					/>
					<span>
						{
							row.original.status === "healthy"
								? "Running"
								: (
									row.original.status === "starting"
										? "Starting"
										: (
											row.original.status === "restarting"
												? "Restarting"
												: (
													row.original.status === "stopping"
														? "Stopping"
														: (
															row.original.status === "unhealthy"
																? "Unhealthy"
																: "Stopped"
														)
												)
										)
								)
						}
					</span>
				</div>
			);
		},
		minSize: 1,
		size: 1,
	},
	{
		id: "node",
		header: () => <span className="w-min h-min select">{ "On Node" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/noeds/[nodeID]
			<Link href="" className="select">{ row.original.node.name }</Link>
		),
		minSize: 1,
		size: 1,
	},
	{
		id: "memory",
		header: () => <span className="w-min h-min select">{ "Memory" }</span>,
		cell: ({ row }) => {
			if(!row.original.memory) {
				return (
					<Skeleton className="h-2 w-24" />
				);
			}

			const percentage = mapValue(row.original.memory.used, 0, row.original.memory.total, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${parseFloat(row.original.memory.used.toFixed(1))} of ${parseFloat(row.original.memory.total.toFixed(1))} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "cpu",
		header: () => <span className="w-min h-min select">{ "CPU" }</span>,
		cell: ({ row }) => {
			if(!row.original.cpu) {
				return (
					<Skeleton className="h-2 w-24" />
				);
			}

			const percentage = mapValue(row.original.cpu.used, 0, row.original.cpu.total, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${parseFloat(row.original.cpu.used.toFixed(1))}% of ${parseFloat(row.original.cpu.total.toFixed(1))}% used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "storage",
		header: () => <span className="w-min h-min select">{ "Storage" }</span>,
		cell: ({ row }) => {
			if(!row.original.storage) {
				return (
					<Skeleton className="h-2 w-24" />
				);
			}

			const percentage = mapValue(row.original.storage.used, 0, row.original.storage.total, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${parseFloat(row.original.storage.used.toFixed(1))} of ${parseFloat(row.original.storage.total.toFixed(1))} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "actions",
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/servers/[serverID]
			<Button size="sm" variant="outline">{ "Manage" }</Button>
		),
		minSize: 1,
		size: 1,
	},
];
