import * as React from "react"

/**
 * The hooks a component calls in one render, in order, without rendering
 * its children. React needs the same hooks in every render of a component;
 * a hook after an early `return` or inside a condition breaks that and the
 * screen fails with «Rendered more hooks than during the previous render».
 * A render to text cannot see it (it renders only once), so this calls the
 * component as a plain function under a stand-in of React's dispatcher
 * that writes down each hook and answers with its first value: the first
 * state, the store's current snapshot, no effects.
 *
 * Compare what it gives for each state of the component (loading, loaded,
 * with and without data): it has to be the same list.
 */
export function hooksOf<P>(component: (props: P) => unknown, props: P): string[] {
  const internals = (React as unknown as Record<string, { H: unknown } | undefined>)
    .__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE
  if (!internals || !("H" in internals)) throw new Error("This version of React does not let the test follow its hooks")
  const calls: string[] = []
  const noop = () => {}
  const contextValue = (context: unknown) => (context as { _currentValue?: unknown })._currentValue
  const hooks: Record<string, (...args: any[]) => unknown> = {
    useState: (initial: unknown) => [typeof initial === "function" ? (initial as () => unknown)() : initial, noop],
    useReducer: (_reducer: unknown, arg: unknown, init?: (arg: unknown) => unknown) => [init ? init(arg) : arg, noop],
    useRef: (current: unknown) => ({ current }),
    useEffect: noop,
    useLayoutEffect: noop,
    useInsertionEffect: noop,
    useImperativeHandle: noop,
    useDebugValue: noop,
    useCallback: (callback: unknown) => callback,
    useMemo: (create: () => unknown) => create(),
    useContext: contextValue,
    use: contextValue,
    useSyncExternalStore: (_subscribe: unknown, getSnapshot: () => unknown) => getSnapshot(),
    useId: () => ":hook-order:",
    useTransition: () => [false, (run: () => void) => run()],
    useDeferredValue: (value: unknown) => value,
    useOptimistic: (value: unknown) => [value, noop],
    useActionState: (_action: unknown, initial: unknown) => [initial, noop, false],
    useEffectEvent: (callback: unknown) => callback,
  }
  const dispatcher = new Proxy(hooks, {
    get(target, name) {
      if (typeof name !== "string") return undefined
      const hook = target[name]
      if (!hook) throw new Error(`hooksOf does not know the hook ${name}`)
      return (...args: unknown[]) => {
        calls.push(name)
        return hook(...args)
      }
    },
  })
  const previous = internals.H
  internals.H = dispatcher
  try {
    component(props)
  } finally {
    internals.H = previous
  }
  return calls
}
