export type Storage = {
    used: number; // Mo
    available: number; // Mo
}

export default async function get_storage(network: string): Promise<Storage> {
  return {
    used: 200 * 1000,
    available: 800 * 1000
  }
}
