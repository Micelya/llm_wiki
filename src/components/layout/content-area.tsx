import { useWikiStore } from "@/stores/wiki-store"
import { ChatPanel } from "@/components/chat/chat-panel"
import { SettingsView } from "@/components/settings/settings-view"
import { SkillsSection } from "@/components/settings/sections/skills-section"
import { SearchView } from "@/components/search/search-view"
import { GraphView } from "@/components/graph/graph-view"
import { HomeView } from "@/components/docbase/home-view"
import { OriginsView } from "@/components/docbase/origins-view"
import { DocumentsView } from "@/components/docbase/documents-view"
import { AttentionView } from "@/components/attention/attention-view"
import { SimilarView } from "@/components/docbase/similar-view"
import { DocumentTypesReview } from "@/components/docbase/document-types-review"
import { EntitiesView } from "@/components/knowledge/entities-view"
import { IdentityDoubtsView } from "@/components/knowledge/doubts-view"
import { MattersView } from "@/components/knowledge/matters-view"
import { PortalsView } from "@/components/portals/portals-view"
import { TemplatesView } from "@/components/docbase/templates-view"
import { ProgramRequestsView } from "@/components/operations/program-requests-view"

export function ContentArea() {
  const activeView = useWikiStore((s) => s.activeView)
  switch (activeView) {
    case "chat":
      return <ChatPanel />
    case "settings":
      return <SettingsView />
    case "skills":
      return <SkillsView />
    case "search":
      return <SearchView />
    case "graph":
      return <GraphView />
    case "origins":
      return <OriginsView />
    case "documents":
      return <DocumentsView />
    case "attention":
      return <AttentionView />
    case "similar":
      return <SimilarView />
    case "document-types":
      return <DocumentTypesReview />
    case "entities":
      return <EntitiesView />
    case "identity-doubts":
      return <IdentityDoubtsView />
    case "matters":
      return <MattersView />
    case "templates":
      return <TemplatesView />
    case "portals":
      return <PortalsView />
    case "program-requests":
      return <ProgramRequestsView />
    case "home":
    default:
      return <HomeView />
  }
}

function SkillsView() {
  return (
    <div className="h-full overflow-y-auto px-8 py-6">
      <div className="mx-auto max-w-3xl">
        <SkillsSection />
      </div>
    </div>
  )
}
