import { ListenEvent } from "./events";
import { ID, Packet, Version } from "./packet";

export function WSListenPacket(events: ListenEvent[]): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSListen,
		data: {
			events,
		},
	} satisfies Packet;
}
