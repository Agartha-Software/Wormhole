import type { NetworkStatus } from "../../ts/NetworkStatus";

export type Network = {
    name: string;
    pods: number;
    status: NetworkStatus
}

/**
 * Lists the networks
 * @returns The list of the networks
 */
export default async function get_networks(): Promise<Network[]> {
  return [
    {
        name: "network1",
        pods: 5,
        status: "online",
    },
    {
        name: "network2",
        pods: 2,
        status: "trouble",
    },
    {
        name: "network3",
        pods: 0,
        status: "offline",
    }
  ]
}
