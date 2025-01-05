import { ID, Packet, Version } from "./packet";

export type WSHandshakeResponseData = {
	challenge: string;
};

export type SWHandshakeRequestData = WSHandshakeResponseData;

export function WSHandshakeResponsePacket(challenge: string): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSHandshakeResponse,
		data: {
			challenge,
		} satisfies WSHandshakeResponseData,
	} satisfies Packet;
}
