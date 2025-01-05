export enum EventType {
	NodeStatus = "NodeStatus",
}

export type NodeStatusEvent = {
	online: boolean;
};

export type ListenEvent = {
	event: EventType;
	daemons: string[];
};

interface EventDataPayloads {
	NodeStatus: NodeStatusEvent;
}

export type EventDataOf<K extends keyof EventDataPayloads> = {
	[P in K]: EventDataPayloads[P];
};

export type EventData = {
	[K in keyof EventDataPayloads]: EventDataOf<K>;
}[keyof EventDataPayloads];

export type Event = {
	event: EventData;
	daemon: string;
};

export type EventOf<K extends keyof EventDataPayloads> = {
	event: EventDataOf<K>;
	daemon: string;
};
