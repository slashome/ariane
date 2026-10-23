import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { addPerson, ApiError, changePerson, fetchPeople, queryKeys, removePerson, type Directory, type PersonChange } from '@/shared/api'
import { toast } from '@/shared/ui'

export function usePeople() {
  return useQuery({ queryKey: queryKeys.people, queryFn: fetchPeople })
}

// One change of the directory; on success the directory is replaced, and the nodes and items, whose
// people and `waiting_for` may have changed, are read again.
function useChange<T>(send: (input: T) => Promise<Directory[]>, done: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: send,
    onSuccess: (directories) => {
      queryClient.setQueryData(queryKeys.people, directories)
      void queryClient.invalidateQueries({ queryKey: queryKeys.nodes })
      void queryClient.invalidateQueries({ queryKey: queryKeys.items })
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      toast.success(done)
    },
    onError: (error) => {
      if (!(error instanceof ApiError && error.status === 409)) {
        toast.error(error.message)
      }
    },
  })
}

export const useAddPerson = () => useChange((person: PersonChange) => addPerson(person), 'Person added')
export const useChangePerson = () => useChange((person: PersonChange) => changePerson(person), 'Person saved')
export const useRemovePerson = () => useChange((person: Pick<PersonChange, 'node' | 'handle'>) => removePerson(person), 'Person removed')
