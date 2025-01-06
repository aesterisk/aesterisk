import { ID, Packet, Version } from "./packet";

export type WSAuthData = {
	user_id: number;
};

export type SWAuthResponseData = {
	success: boolean;
};

export function WSAuthPacket(data: WSAuthData): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSAuth,
		data,
	} satisfies Packet;
}
