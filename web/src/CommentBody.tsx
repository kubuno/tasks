import type { ImgHTMLAttributes } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { useSignedUrl } from '@kubuno/sdk'

/** Markdown image: Drive URLs are persisted bare and signed at render time
 *  (external URLs pass through unchanged). */
function SignedMdImg({ src, ...props }: ImgHTMLAttributes<HTMLImageElement>) {
  const signed = useSignedUrl(typeof src === 'string' ? src : undefined)
  if (!signed) return null
  return <img {...props} src={signed} className="max-w-full max-h-60 rounded-lg my-1" loading="lazy" />
}

/**
 * Safe rendering of a Markdown (GFM) comment: bold/italic, lists, links,
 * images, emoji (unicode). react-markdown does NOT interpret raw HTML, so there
 * is no XSS risk. Links open in a new tab, images are size-bounded.
 */
export default function CommentBody({ body }: { body: string }) {
  return (
    <div className="text-sm text-text-primary break-words [&_p]:my-0.5 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" className="text-primary underline" />,
          img: ({ node: _node, ...props }) => <SignedMdImg {...props} />,
          code: ({ ...props }) => <code {...props} className="bg-surface-2 rounded px-1 py-0.5 text-xs" />,
        }}
      >
        {body}
      </ReactMarkdown>
    </div>
  )
}
