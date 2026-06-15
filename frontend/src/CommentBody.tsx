import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

/**
 * Rendu sûr d'un commentaire en Markdown (GFM) : gras/italique, listes, liens,
 * images, emoji (unicode). react-markdown n'interprète PAS le HTML brut → pas de
 * risque XSS. Liens en nouvel onglet, images bornées.
 */
export default function CommentBody({ body }: { body: string }) {
  return (
    <div className="text-sm text-text-primary break-words [&_p]:my-0.5 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ ...props }) => <a {...props} target="_blank" rel="noopener noreferrer" className="text-primary underline" />,
          img: ({ ...props }) => <img {...props} className="max-w-full max-h-60 rounded-lg my-1" loading="lazy" />,
          code: ({ ...props }) => <code {...props} className="bg-surface-2 rounded px-1 py-0.5 text-xs" />,
        }}
      >
        {body}
      </ReactMarkdown>
    </div>
  )
}
