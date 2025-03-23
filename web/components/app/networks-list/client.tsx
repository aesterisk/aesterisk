"use client";

import { Node } from "@/types/node";
import { DataList } from "../data-list";
import { columns } from "./columns";
import { Check, ChevronsUpDown, Plus } from "lucide-react";
import { NetworkData } from ".";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { DialogClose, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Input } from "@/components/ui/input";
import { useCallback, useMemo, useState } from "react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { cn } from "@/lib/utils";
import { insertNetwork } from "./actions";
import { socketBus } from "@/buses/socket";
import { ID } from "@/packets/packet";

export default function Client({ nodes, networks, teamID }: {
	nodes: Node[];
	networks: NetworkData[];
	teamID: number;
}) {
	const [allNetworks, setAllNetworks] = useState(networks);

	const FormSchema = z.object({
		name: z.string().trim().min(1),
		subnet: z.preprocess((s) => parseInt(z.string().parse(s), 10), z.number().int().min(0).max(255)),
		node: z.number().refine((node) => nodes.some((n) => n.id === node)),
	});

	const form = useForm<z.infer<typeof FormSchema>>({
		resolver: zodResolver(FormSchema),
		defaultValues: {
			name: "",
			subnet: 0,
			node: nodes[0].id,
		},
	});

	const randomPlaceholder = useMemo(() => {
		const names = [
			"Network McNetworkface",
			"Ping Me Maybe",
			"Cloud But Closer",
			"Darth VLAN",
			"Backup-Free Zone",
			"Can't Touch This",
			"Null and Void",
			"Beep Boop",
			"Packet Sniffer",
			"DNSius Maximus",
			"The Ping is a Lie",
			"Temporary Network, Do Not Delete",
			"Do Not Disturb",
			"Gateway to Hell",
			"Old Relic",
			"Lost in the Server Room",
			"CTRL-ALT-DELIGHT",
			"WiFi So Serious?",
			"Not In Service",
			"To Infini- Integer Overflow",
			"Placeholder Name, Please Ignore",
			"Subnet Sloth",
			"Not My Network",
			"Netception",
			"Ping King",
			"Under Maintenance since 2011",
			"Packets In Black",
			"Lord of the Pings",
			"High Latency, Low Morale",
		];

		const random = Math.floor(Math.random() * names.length);

		return names[random];
	}, []);

	const [nodeSelectOpen, setNodeSelectOpen] = useState(false);

	const onSubmit = useCallback(async(data: z.infer<typeof FormSchema>) => {
		const node = nodes.find((n) => n.id === data.node)!;
		const network = await insertNetwork(teamID, data.node, data.name, data.subnet);
		setAllNetworks((n) => n.concat({
			...network,
			node,
		}));
		socketBus.emit(ID.WSSync, node.uuid);
	}, [teamID, nodes]);

	return (
		<DataList
			columns={columns}
			data={allNetworks}
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
				<DialogTitle>{ "Create a New Network" }</DialogTitle>
				<DialogDescription>{ "Networks lets servers communicate with each other" }</DialogDescription>
			</DialogHeader>
			<Form {...form}>
				<form onSubmit={form.handleSubmit(onSubmit)} className="flex flex-col gap-4">
					<FormField
						control={form.control}
						name="name"
						render={
							({ field }) => (
								<FormItem>
									<FormLabel>{ "Network Name" }</FormLabel>
									<FormControl>
										<Input
											placeholder={randomPlaceholder}
											{...field}
										/>
									</FormControl>
									<FormMessage />
								</FormItem>
							)
						}
					/>
					<FormField
						control={form.control}
						name="subnet"
						render={
							({ field }) => (
								<FormItem>
									<FormLabel>{ "Subnet" }</FormLabel>
									<FormControl className="block">
										<code className="flex flex-row items-center text-sm">
											{ "10.133." }
											<Input
												placeholder="123"
												type="number"
												className="w-20 h-8"
												min={0}
												max={255}
												{...field}
											/>
											{ ".0/24" }
										</code>
									</FormControl>
									<FormMessage />
								</FormItem>
							)
						}
					/>
					<FormField
						control={form.control}
						name="node"
						render={
							({ field }) => (
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
																		onSelect={
																			() => {
																				field.onChange(node.id);
																				setNodeSelectOpen(false);
																			}
																		}
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
							)
						}
					/>
					{
						form.formState.isValid
							? (
								<DialogClose asChild>
									<Button type="submit">{ "Create" }</Button>
								</DialogClose>
							)
							: (
								<Button type="submit">{ "Create" }</Button>
							)
					}
				</form>
			</Form>
		</DataList>
	);
}
