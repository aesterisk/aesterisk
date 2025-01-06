export enum Version {
	V0_1_0 = 0,
}

export enum ID {
	WSAuth = 0,
	DSAuth = 1,
	SWHandshakeRequest = 2,
	SDHandshakeRequest = 3,
	WSHandshakeResponse = 4,
	DSHandshakeResponse = 5,
	SWAuthResponse = 6,
	SDAuthResponse = 7,
	WSListen = 8,
	SDListen = 9,
	DSEvent = 10,
	SWEvent = 11,
}

export type Packet = {
	version: Version;
	id: ID;
	data: unknown;
};
