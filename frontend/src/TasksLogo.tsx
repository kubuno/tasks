interface TasksLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Tasks logo (designer artwork, raster). Served by the host from
 *  `/tasks-logo.png`; rendered as a square image so it weighs the same as its
 *  neighbours in the waffle menu. */
export function TasksLogo({ size = 24, className, title = 'Tasks' }: TasksLogoProps) {
  return (
    <img
      src="/tasks-logo.png"
      width={size}
      height={size}
      alt={title}
      className={className}
      style={{ display: 'block', objectFit: 'contain' }}
    />
  )
}

export default TasksLogo
