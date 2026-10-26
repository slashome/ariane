import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { addPersona, ApiError, changePersona, fetchPersonas, queryKeys, removePersona, type PersonaChange, type PersonaDirectory } from '@/shared/api'
import { toast } from '@/shared/ui'

export function usePersonas() {
  return useQuery({ queryKey: queryKeys.personas, queryFn: fetchPersonas })
}

// One change of the personas; on success the directory is replaced, and the nodes and items, whose
// personas may have changed, are read again.
function useChange<T>(send: (input: T) => Promise<PersonaDirectory[]>, done: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: send,
    onSuccess: (directories) => {
      queryClient.setQueryData(queryKeys.personas, directories)
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

export const useAddPersona = () => useChange((persona: PersonaChange) => addPersona(persona), 'Persona added')
export const useChangePersona = () => useChange((persona: PersonaChange) => changePersona(persona), 'Persona saved')
export const useRemovePersona = () => useChange((persona: Pick<PersonaChange, 'node' | 'id'>) => removePersona(persona), 'Persona removed')
