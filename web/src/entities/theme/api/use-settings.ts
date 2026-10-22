import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchSettings, queryKeys, saveTheme, type Settings, type Theme } from '@/shared/api'
import { toast } from '@/shared/ui'

export function useSettings() {
  return useQuery({ queryKey: queryKeys.settings, queryFn: fetchSettings })
}

export function useSaveTheme() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (theme: Theme) => saveTheme(theme),
    onSuccess: (settings: Settings) => {
      queryClient.setQueryData(queryKeys.settings, settings)
      toast.success('Theme saved in settings.toml')
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
    },
    onError: (error) => toast.error(`Could not save the theme: ${error.message}`),
  })
}
