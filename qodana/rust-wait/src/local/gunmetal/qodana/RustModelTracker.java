package local.gunmetal.qodana;

import com.intellij.openapi.application.ApplicationManager;
import com.intellij.openapi.project.DumbService;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.util.Computable;
import com.intellij.platform.backend.observation.ActivityTracker;
import java.util.concurrent.atomic.AtomicLong;
import kotlin.Unit;
import kotlin.coroutines.Continuation;
import kotlinx.coroutines.sync.Mutex;
import org.rust.RsProjectTaskQueueService;
import org.rust.cargo.project.model.CargoProjectsService;
import org.rust.lang.core.resolve2.DefMapService;

/**
 * Qodana's Rust loader returns while DefMaps are still building, and the
 * inspections then run against a half-loaded project. Holding the opening
 * stage for a short, bounded time lets that build finish. The hold is shared
 * across calls: Qodana re-enters this tracker if {@code isInProgress} stays
 * true, and a fresh clock on each call never releases the stage.
 */
public final class RustModelTracker implements ActivityTracker {
    private static final long HOLD_MS = 25_000L;
    private static final AtomicLong STARTED = new AtomicLong(0L);

    @Override
    public String getPresentableName() {
        return "Rust project model";
    }

    @Override
    public Object isInProgress(Project project, Continuation<? super Boolean> completion) {
        return Boolean.valueOf(elapsed() < HOLD_MS && reason(project) != null);
    }

    @Override
    public Object awaitConfiguration(Project project, Continuation<? super Unit> completion) {
        System.out.println("Rust project model: holding analysis up to " + (HOLD_MS / 1000L) + "s for DefMaps");
        String last = "";
        while (elapsed() < HOLD_MS) {
            String why = reason(project);
            if (why == null) {
                break;
            }
            if (!why.equals(last)) {
                System.out.println("Rust project model: " + why);
                last = why;
            }
            try {
                Thread.sleep(250L);
            } catch (InterruptedException interrupted) {
                Thread.currentThread().interrupt();
                break;
            }
        }
        String why = reason(project);
        if (why == null) {
            System.out.println("Rust project model: ready after " + (elapsed() / 1000L) + "s");
        } else {
            System.out.println("Rust project model: releasing analysis after " + (elapsed() / 1000L) + "s (" + why + ")");
        }
        return Unit.INSTANCE;
    }

    private static long elapsed() {
        long now = System.currentTimeMillis();
        STARTED.compareAndSet(0L, now);
        return now - STARTED.get();
    }

    /** {@code null} when the model is ready to inspect. */
    private static String reason(Project project) {
        try {
            return ApplicationManager.getApplication().runReadAction((Computable<String>) () -> reasonInside(project));
        } catch (Throwable failure) {
            return "check failed: " + failure.getClass().getSimpleName();
        }
    }

    private static String reasonInside(Project project) {
        if (project.isDisposed()) {
            return "project disposed";
        }
        if (DumbService.isDumb(project)) {
            return "indexing";
        }
        CargoProjectsService cargo = project.getService(CargoProjectsService.class);
        if (cargo == null || !cargo.getHasAtLeastOneValidProject()) {
            return "Cargo project not loaded";
        }
        RsProjectTaskQueueService queue = project.getService(RsProjectTaskQueueService.class);
        if (queue == null || !queue.isEmpty()) {
            return "Rust task queue is not empty";
        }
        DefMapService defMaps = project.getService(DefMapService.class);
        if (defMaps == null) {
            return "DefMap service missing";
        }
        if (!defMaps.areAllDefMapsUpToDate()) {
            return "DefMaps are still building";
        }
        if (!defMaps.isSafeToAccessDefMaps()) {
            return "DefMaps are not safe to read";
        }
        Mutex lock = defMaps.getDefMapsBuildLock();
        if (lock != null && lock.isLocked()) {
            return "DefMap build lock is held";
        }
        return null;
    }
}
