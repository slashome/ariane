import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { changeCalendar, fetchCalendar, queryKeys, removeCalendarNode, type CalendarOverview } from '@/shared/api'
import { toast } from '@/shared/ui'
import type { CalendarRequest } from '../model/calendar-draft'

export function useCalendar() {
  return useQuery({ queryKey: queryKeys.calendar, queryFn: fetchCalendar })
}

function useRefresh() {
  const queryClient = useQueryClient()
  return (overview?: CalendarOverview) => {
    if (overview) {
      queryClient.setQueryData(queryKeys.calendar, overview)
    }
    for (const queryKey of [queryKeys.calendar, ['roadmap'], queryKeys.repo]) {
      void queryClient.invalidateQueries({ queryKey })
    }
  }
}

async function sendAll(requests: CalendarRequest[]): Promise<CalendarOverview | undefined> {
  let last: CalendarOverview | undefined
  for (const request of requests) {
    last = request.kind === 'remove' ? await removeCalendarNode(request.node) : await changeCalendar(request.change)
  }
  return last
}

export function useSaveCalendar() {
  const refresh = useRefresh()
  return useMutation({
    mutationFn: sendAll,
    onSuccess: refresh,
    onError: (error) => {
      refresh()
      toast.error(error.message)
    },
  })
}

export function useCreateCalendar() {
  const refresh = useRefresh()
  return useMutation({
    mutationFn: (timezone: string) => changeCalendar({ timezone }),
    onSuccess: refresh,
    onError: (error) => {
      toast.error(`Could not create calendar.toml: ${error.message}`)
    },
  })
}
