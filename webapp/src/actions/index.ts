import { defineAction } from 'astro:actions';
import { z } from 'astro/zod';
import get_storage from './get/storage';
import get_networks from './get/networks';

export const server = {
  get_storage: defineAction({
    input: z.string(),
    handler: async (input) => {
      return get_storage(input)
    }
  }),
  get_networks: defineAction({
    handler: async () => {
      return get_networks()
    }
  }),
}