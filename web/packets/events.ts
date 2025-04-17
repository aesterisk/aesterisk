export enum EventType {
	NodeStatus = "NodeStatus",
	ServerStatus = "ServerStatus",
}

export type NodeStatusEvent = {
	online: boolean;
	stats?: {
		used_memory: number;
		total_memory: number;
		cpu: number;
		used_storage: number;
		total_storage: number;
	};
};

export type ServerStatusEvent = {
	statuses: {
		server: number;
		status: "healthy" | "starting" | "restarting" | "stopping" | "offline" | "unhealthy";
		memory?: {
			used: number;
			total: number;
		};
		cpu?: {
			used: number;
			total: number;
		};
		storage?: {
			used: number;
			total: number;
		};
	}[];
};

export type ListenEvent = {
	event: EventType;
	daemons: string[];
};

interface EventDataPayloads {
	NodeStatus: NodeStatusEvent;
	ServerStatus: ServerStatusEvent;
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
