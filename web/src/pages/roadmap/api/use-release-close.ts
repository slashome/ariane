import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { closeRelease, queryKeys } from '@/shared/api'
import { toast } from '@/shared/ui'

export interface CloseTarget {
  node: string
  name: string
}

export function useClosePreview(target: CloseTarget | null, carryOver: boolean) {
  return useQuery({
    queryKey: ['release-close', target?.node ?? '', target?.name ?? '', carryOver],
    queryFn: () => closeRelease({ node: target?.node ?? '', name: target?.name ?? '', carry_over: carryOver, dry_run: true }),
    enabled: target !== null,
    staleTime: 0,
    gcTime: 0,
    retry: false,
  })
}

export function useCloseRelease() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ target, carryOver }: { target: CloseTarget; carryOver: boolean }) =>
      closeRelease({ node: target.node, name: target.name, carry_over: carryOver, dry_run: false }),
    onSuccess: (closing) => {
      for (const queryKey of [queryKeys.nodes, ['roadmap'], queryKeys.items, queryKeys.repo, queryKeys.doctor]) {
        void queryClient.invalidateQueries({ queryKey })
      }
      toast.success(`${closing.release} closed: ${closing.archived.length} archived`)
    },
    onError: (error) => {
      toast.error(`Could not close the release: ${error.message}`)
    },
  })
}
