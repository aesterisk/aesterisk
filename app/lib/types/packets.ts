export enum Version {
	V0_1_0 = 0,
}

export enum ID {
	ASAuth = 0,
	DSAuth = 1,
	SAAuthResponse = 2,
	SDAuthResponse = 3,
	ASListen = 4,
	SDListen = 5,
	DSEvent = 6,
	SAEvent = 7,
}

export type Packet = {
	version: Version;
	id: ID;
	data: unknown;
};

export type ASAuthData = {
	user_id: number;
	public_key: string;
};

export function ASAuthPacket(data: ASAuthData): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.ASAuth,
		data,
	} satisfies Packet;
}

export type SAAuthResponseData = {
	success: boolean;
};

export enum EventType {
	NodesList = "NodesList",
}

export type NodeStatus = {
	id: number;
	status: boolean;
};

export type Event = {
	type: EventType;
	data: unknown;
};

export function ASListenPacket(events: Event[]): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.ASListen,
		data: {
			events: events.map((event) => ({
				[event.type]: event.data,
			})),
		},
	} satisfies Packet;
}
