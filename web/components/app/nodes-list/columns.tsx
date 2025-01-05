"use client";

import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";

import { cn, mapValue } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { NodeData } from ".";

export const columns: ColumnDef<NodeData>[] = [
	{
		id: "name",
		header: () => <span className="w-min h-min select">{ "Node" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/nodes/[nodeID]
			<Link href="" className="select">{ row.original.name }</Link>
		),
	},
	{
		id: "status",
		header: () => <span className="w-min h-min select">{ "Status" }</span>,
		cell: ({ row }) => {
			// todo: show lastActive on hover if offline

			// eslint-disable-next-line no-undefined
			if(row.original.online === undefined) {
				return (
					<Skeleton className="h-2 w-20" />
				);
			}

			return (
				<div className="flex items-center gap-2">
					<div className={cn("rounded-full bg-rose-600 w-2 h-2", row.original.online && "bg-emerald-500")} />
					<span>{ row.original.online ? "Online" : "Offline" }</span>
				</div>
			);
		},
		minSize: 1,
		size: 1,
	},
	{
		id: "servers",
		header: () => <span className="w-min h-min select">{ "Servers" }</span>,
		cell: ({ row }) => {
			if(!row.original.servers) {
				return (
					<Skeleton className="h-2 w-24" />
				);
			}

			const { online, failed, offline } = row.original.servers;

			return (
				<div className="flex flex-row items-center gap-4">
					{
						online > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-emerald-500 w-2 h-2" />
								<span>{ online }</span>
							</div>
						)
					}
					{
						failed > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-rose-600 w-2 h-2" />
								<span>{ failed }</span>
							</div>
						)
					}
					{
						offline > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-primary/20 w-2 h-2" />
								<span>{ offline }</span>
							</div>
						)
					}
				</div>
			);
		},
		minSize: 1,
		size: 1,
	},
	{
		id: "memory",
		header: () => <span className="w-min h-min select">{ "Memory" }</span>,
		cell: ({ row }) => {
			const usage = 13.2;
			const min = 0;
			const max = 16;

			const percentage = mapValue(usage, min, max, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage} of ${max} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "cpu",
		header: () => <span className="w-min h-min select">{ "CPU" }</span>,
		cell: ({ row }) => {
			const usage = 87.3;

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage.toFixed(1)}% used` }</span>
					<Progress value={usage} aria-label={`${usage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "storage",
		header: () => <span className="w-min h-min select">{ "Storage" }</span>,
		cell: ({ row }) => {
			const usage = 241.9;
			const min = 0;
			const max = 256;

			const percentage = mapValue(usage, min, max, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage} of ${max} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "actions",
		cell: ({ row }) => (
		// todo: link to /dash/[teamID]/nodes/[nodeID]
			<Button size="sm" variant="outline">{ "Manage" }</Button>
		),
		minSize: 1,
		size: 1,
	},
];
