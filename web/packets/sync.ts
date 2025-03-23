import { ID, Packet, Version } from "./packet";

export function WSSyncPacket(daemonUuid: string): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSSync,
		data: {
			daemon: daemonUuid,
		},
	} satisfies Packet;
}
