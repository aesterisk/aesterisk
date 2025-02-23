"use client";

import { Node } from "@/types/node";
import { DataList } from "../data-list";
import { columns } from "./columns";
import { NodeData } from ".";
import { useCallback, useMemo, useRef, useState } from "react";
import useEvent from "@/hooks/event";
import { EventOf, EventType } from "@/packets/events";
import { Plus } from "lucide-react";
import { DialogClose, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { StepAccordion, StepAccordionContent, StepAccordionHeader, StepAccordionItem } from "@/components/ui/accordion-steps";
import { Button } from "@/components/ui/button";
import CodeBlock from "../code-block";
import { Input } from "@/components/ui/input";
import { z } from "zod";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Form, FormControl, FormField, FormItem } from "@/components/ui/form";
import { insertNode } from "./actions";
import { Textarea } from "@/components/ui/textarea";

export default function Client({ nodes, teamID }: {
	nodes: Node[];
	teamID: number;
}) {
	const [allNodes, setAllNodes] = useState(nodes);

	const [nodeData, setNodeData] = useState<NodeData[]>(allNodes.map((node) => ({
		uuid: node.uuid,
		name: node.name,
		// eslint-disable-next-line no-undefined
		lastActive: node.lastActive ?? undefined,
	})));

	const nodeUuids = useMemo(() => allNodes.map((node) => node.uuid), [allNodes]);

	const [createdUuid, setCreatedUuid] = useState("Loading...");
	const [currentOpen, setCurrentOpen] = useState(0);
	const [currentStep, setCurrentStep] = useState(0);
	const [buttonLabel, setButtonLabel] = useState("Next");
	const [addSuccess, setAddSuccess] = useState(false);

	const setOpen = useCallback((to: number) => {
		if(to === 4) {
			setButtonLabel(addSuccess ? "Done" : "Waiting for Connection...");
		} else if(to === 2) {
			setButtonLabel("Save");
		} else {
			setButtonLabel("Next");
		}

		setCurrentOpen(to);
	}, [addSuccess]);

	const updateStatus = useCallback((event: EventOf<EventType.NodeStatus>) => {
		setNodeData((data) => data.map((node) => ({
			...node,
			online: node.uuid === event.daemon ? event.event.NodeStatus.online : node.online,
			memory: node.uuid === event.daemon
				? {
					used: event.event.NodeStatus.stats?.used_memory,
					total: event.event.NodeStatus.stats?.total_memory,
				}
				: node.memory,
			cpu: node.uuid === event.daemon ? event.event.NodeStatus.stats?.cpu : node.cpu,
			storage: node.uuid === event.daemon
				? {
					used: event.event.NodeStatus.stats?.used_storage,
					total: event.event.NodeStatus.stats?.total_storage,
				}
				: node.storage,
		})));

		if(event.daemon === createdUuid && event.event.NodeStatus.online && !addSuccess) {
			setAddSuccess(true);
			setCurrentStep(5);
			setOpen(4);
			setButtonLabel("Done");
		}
	}, [createdUuid, addSuccess, setOpen]);

	useEvent(EventType.NodeStatus, updateStatus, nodeUuids);

	const hasCreatedUuid = useRef(false);

	const FormSchema = z.object({
		name: z.string().trim().min(1),
		key: z.string().trim().regex(/-{5}BEGIN PUBLIC KEY-{5}(?:\n.{64}){6}\n.{8}\n-{5}END PUBLIC KEY-{5}/gmu),
	});

	const form = useForm<z.infer<typeof FormSchema>>({
		resolver: zodResolver(FormSchema),
		defaultValues: {
			name: "",
			key: "",
		},
	});

	const buttonDisabled = useCallback(() => {
		switch(currentOpen) {
			case 0:
				return false;
			case 1: {
				const nameNotOkay = Boolean(form.formState.errors.name); // zod name validation here
				if(nameNotOkay && currentStep > 1) {
					setCurrentStep(1);
				}
				return nameNotOkay;
			}
			case 2: {
				const keyNotOkay = Boolean(form.formState.errors.key); // zod key validation here
				if(keyNotOkay && currentStep > 2) {
					setCurrentStep(2);
				}
				return keyNotOkay;
			}
			case 3:
				return false;
			case 4:
				return !addSuccess; // listen for events
			default:
				return false;
		}
	}, [currentOpen, currentStep, form.formState.errors.name, form.formState.errors.key, addSuccess]);

	const update = useCallback(async(to: number, onlyBack?: boolean) => {
		if(to < 5) {
			if(onlyBack) {
				if(to <= currentStep) {
					if((currentStep < 3 && to < 3) || (currentStep >= 3 && to >= 3)) {
						setOpen(to);
					}
				}
			} else {
				setOpen(to);
				if(to > currentStep) {
					setCurrentStep(to);
				}
			}

			if(to === 3 && !hasCreatedUuid.current) {
				hasCreatedUuid.current = true;
				const { name, key } = form.getValues();
				const node = await insertNode(teamID, name, key);
				setCreatedUuid(node.uuid);
				setAllNodes((n) => n.concat(node));
				setNodeData((d) => d.concat({
					uuid: node.uuid,
					name: node.name,
					// eslint-disable-next-line no-undefined
					lastActive: node.lastActive ?? undefined,
				} satisfies NodeData));
			}
		}
	}, [currentStep, form, teamID, hasCreatedUuid, setOpen]);

	const next = () => {
		update(currentOpen + 1);
	};

	const open = useCallback(() => {
		setCurrentStep(0);
		update(0);
	}, [update]);

	const getState = useCallback((step: number) => {
		if(step === currentStep) {
			return "pending";
		} else if(step < currentStep) {
			return "done";
		}

		return "not-started";
	}, [currentStep]);

	const randomPlaceholder = useMemo(() => {
		const names = [
			"Server McServerface",
			"Ping Me Maybe",
			"Cloud But Closer",
			"Darth VLAN",
			"Serverus Snape",
			"Serverous Black",
			"Backup-Free Zone",
			"Can't Touch This",
			"Dora the Internet Explorer",
			"Blue Screen Enjoyer",
			"WeRanOutOfNames",
			"Cache Me If You Can",
			"Why So Serial?",
			"Null and Void",
			"Kernel Panic at the Disco",
			"I Host Therefore I Am",
			"Still Booting",
			"Don't Reboot Me Bro",
			"Beep Boop",
			"Packet Sniffer",
			"DNSius Maximus",
			"The Ping is a Lie",
			"Big Server Energy",
			"Overclocked Potato",
			"Runtime Terror",
			"Temporary Server, Do Not Delete",
			"Do Not Disturb",
			"Gateway to Hell",
			"Old Relic",
			"Lost in the Server Room",
			"CTRL-ALT-DELIGHT",
			"Kernel Sanders",
			"WiFi So Serious?",
			"Not In Service",
			"To Infini- Integer Overflow",
			"Placeholder Name, Please Ignore",
			"Subnet Sloth",
			"Not My Server",
			"Serverception",
			"Ping King",
			"Under Maintenance since 2011",
			"Packets In Black",
			"Lord of the Pings",
			"Static Shocker",
			"Rebootycall",
			"Deploy and Pray",
			"High Latency, Low Morale",
		];

		const random = Math.floor(Math.random() * names.length);

		return names[random];
	}, []);

	return (
		<DataList
			columns={columns}
			data={nodeData}
			actionLabel={
				(
					<>
						<Plus />
						{ "Add a Node" }
					</>
				)
			}
			onClick={open}
			dialogTrigger
		>
			<DialogHeader>
				<DialogTitle>{ "Add a Node" }</DialogTitle>
				<DialogDescription>{ "Follow the steps to set up your node with Aesterisk" }</DialogDescription>
			</DialogHeader>
			<Form {...form}>
				<form onSubmit={form.handleSubmit(() => {})} className="flex flex-col">
					<StepAccordion type="single" value={`${currentOpen}`} onValueChange={(v) => update(parseInt(v, 10), true)}>
						<StepAccordionItem value="0">
							<StepAccordionHeader state={getState(0)}>
								{ "Install Aesterisk Daemon" }
							</StepAccordionHeader>
							<StepAccordionContent>
								<p>{ "SSH into the computer you want to install Aesterisk on, and run our interactive installer:" }</p>
								<CodeBlock className="w-full">
									{ "sudo " }
									<span className="text-emerald-500">{ "curl" }</span>
									{ " -fsSL " }
									<span className="text-yellow-400">{ "https://get.aesterisk.io" }</span>
									{ " | " }
									<span className="text-emerald-500">{ "sh" }</span>
								</CodeBlock>
							</StepAccordionContent>
						</StepAccordionItem>
						<StepAccordionItem value="1">
							<StepAccordionHeader state={getState(1)}>
								{ "Name your Node" }
							</StepAccordionHeader>
							<StepAccordionContent>
								<p className="mb-4">
									<span>{ "VPS machines are usually named " }</span>
									<code className="bg-muted rounded p-0.5">{ "[Provider]-[ID]" }</code>
									<span>{ " by enterprises, although you can name it whatever you prefer." }</span>
								</p>
								<FormField
									control={form.control}
									name="name"
									render={
										({ field }) => (
											<FormItem>
												<FormControl>
													<Input
														placeholder={randomPlaceholder}
														{...field}
													/>
												</FormControl>
											</FormItem>
										)
									}
								/>
							</StepAccordionContent>
						</StepAccordionItem>
						<StepAccordionItem value="2">
							<StepAccordionHeader state={getState(2)}>
								{ "Exchange Key" }
							</StepAccordionHeader>
							<StepAccordionContent>
								<span>{ "Start the daemon once (it will exit immediately, don't worry):" }</span>
								<CodeBlock className="w-full">
									{ "sudo " }
									<span className="text-yellow-400">{ "aesteriskd" }</span>
								</CodeBlock>
								<p className="mb-4">
									<span>{ "once to generate the keys, and copy the contents of " }</span>
									<code className="bg-muted rounded p-0.5">{ "/etc/aesterisk/daemon.pub" }</code>
									<span>{ " and paste it here." }</span>
								</p>
								<FormField
									control={form.control}
									name="key"
									render={
										({ field }) => (
											<FormItem>
												<FormControl>
													<Textarea
														placeholder="Paste your key here"
														className="resize-none"
														{...field}
													/>
												</FormControl>
											</FormItem>
										)
									}
								/>
							</StepAccordionContent>
						</StepAccordionItem>
						<StepAccordionItem value="3">
							<StepAccordionHeader state={getState(3)}>
								{ "Configure Daemon" }
							</StepAccordionHeader>
							<StepAccordionContent>
								<p className="mb-4">
									<span>{ "Copy your unique Daemon ID and add it to the configuration file located at " }</span>
									<code className="bg-muted rounded p-0.5">{ "/etc/aesterisk/config.toml" }</code>
								</p>
								<CodeBlock copyString={createdUuid} className="w-full">
									<p className="text-emerald-500">{ "[daemon]" }</p>
									<p>
										{ "uuid = \"" }
										<span className="text-yellow-400">{ createdUuid }</span>
										{ "\"" }
									</p>
								</CodeBlock>
							</StepAccordionContent>
						</StepAccordionItem>
						<StepAccordionItem value="4" last>
							<StepAccordionHeader state={getState(4)}>
								{ "Start Daemon" }
							</StepAccordionHeader>
							<StepAccordionContent>
								<p>
									{ "Start the Aesterisk Daemon with `sudo systemctl enable --now aesterisk` and it should successfully connect. If not, see the troubleshooting guide." }
								</p>
							</StepAccordionContent>
						</StepAccordionItem>
					</StepAccordion>
					<div className="bg-transparent h-[17px]" />
					{ /* h-4 + 1px, fixes blurryness issues as the StepAccordion is of an odd height */ }
					{
						currentStep === 5
							? (
								<DialogClose asChild>
									<Button onClick={next} disabled={buttonDisabled()} className="w-full">
										{ buttonLabel }
									</Button>
								</DialogClose>
							)
							: (
								<Button onClick={next} disabled={buttonDisabled()} className="w-full">
									{ buttonLabel }
								</Button>
							)
					}
				</form>
			</Form>
		</DataList>
	);
}
