import { Outlet } from '@tanstack/react-router'

export function RootLayout() {
  return (
    <div className="flex min-h-screen justify-center px-6 pb-20 pt-12 sm:px-5 sm:pt-10 lg:px-8">
      <Outlet />
    </div>
  )
}

