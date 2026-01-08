import type { Network } from "./networks";
import get_networks from "./networks";

export default async function get_network(name: string): Promise<Network | undefined> {
  return (await get_networks()).find(nw => nw.name === name)
}