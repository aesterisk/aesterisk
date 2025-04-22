"use client";

import { useCallback, useMemo, useState } from "react";
import { ServerData } from ".";
import { z } from "zod";
import { Node } from "@/types/node";
import { ControllerRenderProps, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { socketBus } from "@/buses/socket";
import { ID } from "@/packets/packet";
import { DataList } from "../data-list";
import { Check, ChevronsUpDown, Plus } from "lucide-react";
import { DialogClose, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { cn } from "@/lib/utils";
import { columns } from "./columns";
import { insertServer } from "./actions";
import { EventOf, EventType } from "@/packets/events";
import useEvent from "@/hooks/event";

export default function Client({ nodes, servers, teamID }: {
	nodes: Node[];
	servers: ServerData[];
	teamID: number;
}) {
	const [allServers, setAllServers] = useState(servers);

	const [serverData, setServerData] = useState(allServers);

	const nodeUuids = useMemo(() => nodes.map((node) => node.uuid), [nodes]);

	const updateStatus = useCallback((event: EventOf<EventType.ServerStatus>) => {
		setServerData((data) => data.map((server) => {
			const status = event.event.ServerStatus;
			if(status.server !== server.id) return server;

			return {
				...server,
				status: status.status,
				memory: status.memory,
				cpu: status.cpu,
				storage: status.storage,
			};
		}));
	}, []);

	useEvent(EventType.ServerStatus, updateStatus, nodeUuids);

	const FormSchema = z.object({
		name: z.string().trim().min(1),
		tag: z.preprocess((s) => parseInt(z.string().parse(s), 10), z.number().int().min(1)),
		node: z.number().refine((node) => nodes.some((n) => n.id === node)),
	});

	const form = useForm<z.infer<typeof FormSchema>>({
		resolver: zodResolver(FormSchema),
		defaultValues: {
			name: "",
			node: nodes[0].id,
			tag: 1,
		},
	});

	const randomPlaceholder = useMemo(() => {
		const names = ["TODO: Add random names"];

		const random = Math.floor(Math.random() * names.length);

		return names[random];
	}, []);

	const [nodeSelectOpen, setNodeSelectOpen] = useState(false);

	const onSubmit = useCallback(async(data: z.infer<typeof FormSchema>) => {
		const node = nodes.find((n) => n.id === data.node);
		if(!node) return;

		const server = await insertServer(teamID, data.node, data.name, data.tag);
		setAllServers((s) => s.concat({
			...server,
			node,
		}));
		socketBus.emit(ID.WSSync, node.uuid);
	}, [teamID, nodes]);

	const onSelect = useCallback((id: string) => {
		form.setValue("node", parseInt(id, 10));
		setNodeSelectOpen(false);
	}, [form]);

	const renderNameField = useCallback(({ field }: {
		field: ControllerRenderProps<{
			name: string;
			tag: number;
			node: number;
		}, "name">;
	}) => (
		<FormItem>
			<FormLabel>{ "Server Name" }</FormLabel>
			<FormControl>
				<Input
					placeholder={randomPlaceholder}
					{...field}
				/>
			</FormControl>
			<FormMessage />
		</FormItem>
	), [randomPlaceholder]);

	const renderTagField = useCallback(({ field }: {
		field: ControllerRenderProps<{
			name: string;
			tag: number;
			node: number;
		}, "tag">;
	}) => (
		<FormItem>
			<FormLabel>{ "Tag" }</FormLabel>
			<FormControl className="block">
				<Input
					placeholder="123"
					type="number"
					className="w-20 h-8"
					min={1}
					{...field}
				/>
			</FormControl>
			<FormMessage />
		</FormItem>
	), []);

	const renderNodeField = useCallback(({ field }: {
		field: ControllerRenderProps<{
			name: string;
			tag: number;
			node: number;
		}, "node">;
	}) => (
		// skipcq: JS-0415
		<FormItem className="flex flex-col">
			<FormLabel>{ "Node" }</FormLabel>
			<FormControl>
				<Popover open={nodeSelectOpen} onOpenChange={setNodeSelectOpen}>
					<PopoverTrigger asChild>
						<Button variant="outline" role="combobox" aria-expanded={nodeSelectOpen} className="w-[200px] justify-between" ref={field.ref} disabled={field.disabled}>
							{ /* todo: add status indicator for node */ }
							{ nodes.find((n) => n.id === field.value)?.name }
							<ChevronsUpDown className="opacity-50" />
						</Button>
					</PopoverTrigger>
					<PopoverContent className="w-[200px] p-0">
						<Command>
							<CommandInput placeholder="Search node..." />
							<CommandList>
								<CommandEmpty>{ "No nodes found." }</CommandEmpty>
								<CommandGroup>
									{
										nodes.map((node) => (
											<CommandItem
												key={node.id}
												value={node.id.toString()}
												onSelect={onSelect}
											>
												{ node.name }
												<Check className={cn("ml-auto", field.value === node.id ? "opacity-100" : "opacity-0")} />
											</CommandItem>
										))
									}
								</CommandGroup>
							</CommandList>
						</Command>
					</PopoverContent>
				</Popover>
			</FormControl>
			<FormMessage />
		</FormItem>
	), [nodeSelectOpen, nodes, onSelect]);

	return (
		<DataList
			columns={columns}
			data={serverData}
			actionLabel={
				(
					<>
						<Plus />
						{ "Create New" }
					</>
				)
			}
			dialogTrigger
		>
			<DialogHeader>
				<DialogTitle>{ "Create a New Server" }</DialogTitle>
				<DialogDescription>{ "Placeholder description" }</DialogDescription>
			</DialogHeader>
			<Form {...form}>
				<form onSubmit={form.handleSubmit(onSubmit)} className="flex flex-col gap-4">
					<FormField
						control={form.control}
						name="name"
						render={renderNameField}
					/>
					<FormField
						control={form.control}
						name="tag"
						render={renderTagField}
					/>
					<FormField
						control={form.control}
						name="node"
						render={renderNodeField}
					/>
					{
						form.formState.isValid
							? (
								<DialogClose asChild>
									<Button type="submit">{ "Create" }</Button>
								</DialogClose>
							)
							: (
								<Button type="button" disabled>{ "Create" }</Button>
							)
					}
				</form>
			</Form>
		</DataList>
	);
}
