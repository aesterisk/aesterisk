import { EncryptJWT, KeyLike, exportPKCS8, exportSPKI, generateKeyPair, importSPKI, jwtDecrypt } from "jose";
import { Packet } from "@/types/packets";
import { cache } from "react";

const getServerPublicKey = cache(async() => await importSPKI(process.env.NEXT_PUBLIC_SERVER_PUBLIC_KEY!, "RSA-OAEP"));

export async function encryptPacket(packet: object): Promise<string> {
	return await new EncryptJWT({ p: packet })
		.setProtectedHeader({
			alg: "RSA-OAEP",
			enc: "A256GCM",
		})
		.setIssuedAt()
		.setIssuer("aesterisk/web")
		.setExpirationTime("1 minute")
		.encrypt(await getServerPublicKey());
}

export async function decryptPacket(packet: string, key: KeyLike): Promise<Packet> {
	const jwe = await jwtDecrypt(packet, key, {
		issuer: "aesterisk/server",
		keyManagementAlgorithms: ["RSA-OAEP"],
		contentEncryptionAlgorithms: ["A256GCM"],
	});

	return jwe.payload.p as Packet;
}

export async function generateKeys() {
	const pair = await generateKeyPair("RS256");
	return {
		publicKey: await exportSPKI(pair.publicKey),
		privateKey: await exportPKCS8(pair.privateKey),
	};
}
